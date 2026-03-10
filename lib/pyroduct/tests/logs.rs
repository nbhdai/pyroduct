//! Tests for the FFI logging system (host side + callback safety).
//!
//! These exercise `log_callback`, `create_log`, `destroy_log`, and the
//! `CATCH_LOG_SENDER` / `LOG_SENDERS` dispatch paths for soundness.

use std::sync::atomic::{AtomicUsize, Ordering};

use pyroduct::module::capability::{create_log, destroy_log, log_callback};

// =============================================================================
// Helpers
// =============================================================================

/// Call `log_callback` with a Rust string — wraps the unsafe pointer dance.
unsafe fn send_log(lib_id: i64, span_id: u64, msg: &str) {
    unsafe { log_callback(lib_id, span_id, msg.as_ptr(), msg.len()) };
}

// =============================================================================
// Basic delivery
// =============================================================================

#[tokio::test]
async fn test_log_delivery_single_message() {
    let lib_id: i64 = 10_000;
    let span_id: u64 = 1;
    let mut rx = create_log(lib_id, span_id, 16);

    unsafe { send_log(lib_id, span_id, "hello from ffi\n") };

    let msg = rx.recv().await.expect("should receive a message");
    assert_eq!(msg, "hello from ffi");

    destroy_log(lib_id, span_id);
}

#[tokio::test]
async fn test_log_delivery_multiple_messages() {
    let lib_id: i64 = 10_001;
    let span_id: u64 = 1;
    let mut rx = create_log(lib_id, span_id, 64);

    for i in 0..10 {
        unsafe { send_log(lib_id, span_id, &format!("msg {i}\n")) };
    }

    for i in 0..10 {
        let msg = rx.recv().await.expect("should receive message");
        assert_eq!(msg, format!("msg {i}"));
    }

    destroy_log(lib_id, span_id);
}

// =============================================================================
// Routing: different span IDs go to different receivers
// =============================================================================

#[tokio::test]
async fn test_log_routing_by_span_id() {
    let lib_id: i64 = 10_002;
    let mut rx_a = create_log(lib_id, 100, 16);
    let mut rx_b = create_log(lib_id, 200, 16);

    unsafe {
        send_log(lib_id, 100, "for A\n");
        send_log(lib_id, 200, "for B\n");
    }

    assert_eq!(rx_a.recv().await.unwrap(), "for A");
    assert_eq!(rx_b.recv().await.unwrap(), "for B");

    destroy_log(lib_id, 100);
    destroy_log(lib_id, 200);
}

#[tokio::test]
async fn test_log_routing_by_library_id() {
    let span_id: u64 = 1;
    let mut rx_1 = create_log(10_003, span_id, 16);
    let mut rx_2 = create_log(10_004, span_id, 16);

    unsafe {
        send_log(10_003, span_id, "lib 1\n");
        send_log(10_004, span_id, "lib 2\n");
    }

    assert_eq!(rx_1.recv().await.unwrap(), "lib 1");
    assert_eq!(rx_2.recv().await.unwrap(), "lib 2");

    destroy_log(10_003, span_id);
    destroy_log(10_004, span_id);
}

// =============================================================================
// Destroy / channel lifecycle
// =============================================================================

#[tokio::test]
async fn test_destroy_log_closes_receiver() {
    let lib_id: i64 = 10_010;
    let span_id: u64 = 1;
    let mut rx = create_log(lib_id, span_id, 16);

    destroy_log(lib_id, span_id);

    // After destroy, receiver should see channel closed
    assert!(rx.recv().await.is_none());
}

#[tokio::test]
async fn test_log_after_destroy_does_not_panic() {
    let lib_id: i64 = 10_011;
    let span_id: u64 = 1;
    let _rx = create_log(lib_id, span_id, 16);
    destroy_log(lib_id, span_id);

    // Sending to a destroyed channel must not segfault or panic
    unsafe { send_log(lib_id, span_id, "ghost message\n") };
}

#[tokio::test]
async fn test_log_to_nonexistent_channel_does_not_panic() {
    // No channel was ever created for this ID pair
    unsafe { send_log(99_999, 99_999, "nowhere\n") };
}

#[tokio::test]
async fn test_double_destroy_does_not_panic() {
    let lib_id: i64 = 10_012;
    let span_id: u64 = 1;
    let _rx = create_log(lib_id, span_id, 16);

    destroy_log(lib_id, span_id);
    destroy_log(lib_id, span_id); // second destroy is a no-op
}

// =============================================================================
// Back-pressure: channel full
// =============================================================================

#[tokio::test]
async fn test_log_channel_full_drops_message() {
    let lib_id: i64 = 10_013;
    let span_id: u64 = 1;
    // buffer = 2 means capacity of 2
    let mut rx = create_log(lib_id, span_id, 2);

    unsafe {
        send_log(lib_id, span_id, "msg 1\n");
        send_log(lib_id, span_id, "msg 2\n");
        // This one should be dropped (channel full, try_send fails)
        send_log(lib_id, span_id, "msg 3\n");
    }

    let m1 = rx.recv().await.unwrap();
    let m2 = rx.recv().await.unwrap();
    assert_eq!(m1, "msg 1");
    assert_eq!(m2, "msg 2");

    destroy_log(lib_id, span_id);
}

// =============================================================================
// Receiver dropped before sender (closed channel path)
// =============================================================================

// #[tokio::test]
// async fn test_log_receiver_dropped_before_send() {
//     let lib_id: i64 = 10_014;
//     let span_id: u64 = 1;
//     let rx = create_log(lib_id, span_id, 16);

//     // Drop receiver — simulates the host side losing interest
//     drop(rx);

//     // Callback should handle the closed channel gracefully (no panic)
//     unsafe { send_log(lib_id, span_id, "orphan message\n") };

//     // Cleanup
//     // destroy_log(lib_id, span_id);
// }

// =============================================================================
// Concurrent safety: hammer from multiple threads
// =============================================================================

#[tokio::test]
async fn test_log_concurrent_writes() {
    let lib_id: i64 = 10_015;
    let span_id: u64 = 1;
    let mut rx = create_log(lib_id, span_id, 1024);

    let count = 100;
    let received = std::sync::Arc::new(AtomicUsize::new(0));

    let handles: Vec<_> = (0..count)
        .map(|i| {
            tokio::task::spawn_blocking(move || {
                let msg = format!("thread {i}\n");
                unsafe { send_log(lib_id, span_id, &msg) };
            })
        })
        .collect();

    for h in handles {
        h.await.unwrap();
    }

    // Drain all messages
    let r = received.clone();
    let drain = tokio::spawn(async move {
        while let Ok(msg) = tokio::time::timeout(
            std::time::Duration::from_millis(200),
            rx.recv(),
        )
        .await
        {
            if msg.is_none() {
                break;
            }
            r.fetch_add(1, Ordering::Relaxed);
        }
    });

    drain.await.unwrap();
    assert_eq!(received.load(Ordering::Relaxed), count);

    destroy_log(lib_id, span_id);
}

// =============================================================================
// Zero-length pointer safety
// =============================================================================

#[tokio::test]
async fn test_log_zero_length_slice() {
    let lib_id: i64 = 10_016;
    let span_id: u64 = 1;
    let mut rx = create_log(lib_id, span_id, 16);

    // Pass a valid pointer but zero length — from_raw_parts(ptr, 0) is safe
    let dummy: u8 = 0;
    unsafe { log_callback(lib_id, span_id, &dummy as *const u8, 0) };

    let msg = rx.recv().await.unwrap();
    assert_eq!(msg, "");

    destroy_log(lib_id, span_id);
}