use super::safe_async::execute_safe_async;
use super::safe_io::get_input;
use super::safe_lifecycle::{execute_safe_init, execute_safe_reset};
use crate::CapIdentity;
use crate::capability_host::ffi::FfiBorrowedFutureResult;
use crate::errors::FfiError;
use crate::host::ffi_bridge::{AsyncExecFuture, ExecutionResultBridge, InitResultBridge};
use crate::module_capability::panic::register_ffi_panic_hook;
use std::path::Path;
use std::ptr;

// A simple type for serialization testing
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Debug, PartialEq)]
#[rkyv(compare(PartialEq), derive(Debug))]
struct TestInput {
    id: Vec<u32>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
struct TestConfig {
    id: Vec<u32>,
}

#[tracing_test::traced_test]
#[test]
fn test_null_pointer_protection() {
    // Attempt to deserialize from a null pointer
    let result = unsafe { get_input::<TestInput>(ptr::null(), 10) };

    match result {
        Err(FfiError::NullPointer(_)) => assert!(true),
        _ => panic!("Should have caught NullPointer, got {:?}", result),
    }
}

#[tracing_test::traced_test]
#[test]
fn test_zero_length_protection() {
    // Valid pointer but zero length
    let data = vec![1u8, 2, 3];
    let result = unsafe { get_input::<TestInput>(data.as_ptr(), 0) };

    match result {
        Err(FfiError::ZeroLength(_)) => assert!(true),
        _ => panic!("Should have caught ZeroLength, got {:?}", result),
    }
}

#[tracing_test::traced_test]
#[test]
fn test_deserialization_garbage_data() {
    // Pointer to random garbage bytes
    let garbage = vec![0u8, 255, 12, 33];
    let result = unsafe { get_input::<TestInput>(garbage.as_ptr(), garbage.len()) };

    match result {
        Err(FfiError::ValidationFailed(_, _)) => assert!(true),
        Err(FfiError::DeserializationFailed(_, _)) => assert!(true), // Depends on where it fails
        _ => panic!("Should have failed deserialization, got {:?}", result),
    }
}

#[tracing_test::traced_test]
#[test]
fn test_lifecycle_panic_recovery() {
    // Serialize valid config to pass the input check
    let config = TestConfig { id: vec![1] };
    let bytes = serde_json::to_vec(&config).unwrap();

    // A closure that deliberately panics
    let panicking_init = |_config: TestConfig| -> usize {
        panic!("Deliberate crash in init");
    };

    // Register the panic hook so FFI can capture the message
    register_ffi_panic_hook();

    let result = unsafe { execute_safe_init(bytes.as_ptr(), bytes.len(), panicking_init) };

    // Check that we got an error tag (1) instead of crashing the test runner
    assert_eq!(result.tag, 1);

    // Optional: Deserialize the error output to verify it contains the panic message
    // This requires replicating the error deserialization logic from ffi_bridge.rs
}

#[tracing_test::traced_test]
#[tokio::test]
async fn test_async_logic_panic_recovery() {
    // Register hook
    register_ffi_panic_hook();

    // Create an FFI future wrapper around a panicking async block
    let ffi_future = execute_safe_async(async {
        panic!("Deliberate crash in async logic");
        // Unreachable return to satisfy types
        #[allow(unreachable_code)]
        TestInput { id: vec![0] }
    });

    // We expect the future to complete (not crash) but yield an error output
    match ffi_future {
        FfiBorrowedFutureResult::Future(fut) => {
            let result = fut.await;
            assert_eq!(result.tag, 1); // 1 = Error
        }
        _ => panic!("Should have returned a Future variant"),
    }
}

#[tracing_test::traced_test]
#[test]
fn test_bridge_handles_lifecycle_panic() {
    register_ffi_panic_hook();

    // 1. Prepare valid input
    let config = TestConfig { id: vec![1] };
    let bytes = serde_json::to_vec(&config).expect("failed to serialize");

    // 2. Run the "Plugin" side logic which panics
    // This simulates the plugin crashing during init
    let raw_ffi_result = unsafe {
        execute_safe_init(bytes.as_ptr(), bytes.len(), |_: TestConfig| -> usize {
            panic!("Catastrophic failure in plugin init");
        })
    };

    // 3. Run the "Host" side bridge logic
    // This consumes the raw FFI result and attempts to reconstruct the error
    let host_result = unsafe {
        InitResultBridge::from_ffi(raw_ffi_result, &CapIdentity::from(Path::new("test_cap")))
    };
    tracing::info!(?host_result, "Result");

    // 4. Verify the Host sees the specific LogicPanicked error
    match host_result {
        Err(e) => {
            // Need to match against inner FfiError to verify
            // Since we can't easily destructure PyroductError in tests without public accessors or manual matching:
            let msg = e.to_string();
            assert!(msg.contains("Catastrophic failure in plugin init"));
            assert!(msg.contains("logic panic"));
        }
        _ => panic!(
            "Expected Host to decode a LogicPanicked error, got: {:?}",
            host_result
        ),
    }
}

#[tracing_test::traced_test]
#[test]
fn test_bridge_handles_null_pointer_input() {
    // 1. Run the "Plugin" side logic with invalid input (null pointer)
    // This wraps `execute_safe_reset`
    let raw_ffi_result = unsafe { execute_safe_reset::<usize, _>(ptr::null_mut(), |_| {}) };

    // 2. Run the "Host" side bridge logic
    let host_result = unsafe {
        ExecutionResultBridge::expected_null_from_ffi(
            raw_ffi_result,
            &CapIdentity::from(Path::new("test_cap")),
        )
    };

    // 3. Verify Host sees the NullPointer error
    match host_result {
        Err(e) => {
            let msg = e.to_string();
            assert!(msg.contains("null pointer"));
        }
        _ => panic!(
            "Expected Host to decode NullPointer error, got: {:?}",
            host_result
        ),
    }
}

#[tracing_test::traced_test]
#[tokio::test]
async fn test_bridge_handles_async_panic() {
    register_ffi_panic_hook();

    // 1. Run "Plugin" side async logic that panics
    let ffi_future_result = execute_safe_async(async {
        panic!("Async logic crash");
        #[allow(unreachable_code)]
        Vec::<u8>::new()
    });

    // 2. Wrap it in the Host Bridge Future
    // AsyncExecFuture handles polling the C-compatible future and converting the result
    let host_future = AsyncExecFuture::new(
        ffi_future_result,
        &CapIdentity::from(Path::new("test_cap")),
    );

    // 3. Await it like a normal Rust future
    let result = host_future.await;

    // 4. Verify the error
    match result {
        Err(e) => {
            let msg = e.to_string();
            assert!(msg.contains("Async logic crash"));
        }
        _ => panic!(
            "Expected LogicPanicked from async bridge, got: {:?}",
            result
        ),
    }
}

#[tracing_test::traced_test]
#[test]
fn test_bridge_handles_deserialization_failure() {
    // 1. Pass garbage data to init
    let garbage = vec![0u8, 255, 12, 33]; // Invalid rkyv data for a vector

    let raw_ffi_result = unsafe {
        execute_safe_init(garbage.as_ptr(), garbage.len(), |_: TestConfig| -> usize {
            0
        })
    };

    // 2. Decode on host
    let host_result = unsafe {
        InitResultBridge::from_ffi(raw_ffi_result, &CapIdentity::from(Path::new("test_cap")))
    };

    // 3. Verify
    match host_result {
        Err(e) => {
            let msg = e.to_string();
            assert!(msg.contains("expected value at line 1 column 1"));
        }
        _ => panic!(
            "Expected expected value at line 1 column 1, got: {:?}",
            host_result
        ),
    }
}
