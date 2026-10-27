//! Exhaustive interleaving checks for the refcount lifecycle.
//!
//! Run with: `cargo test --features debug --test loom_buf --release`
//!
//! **Why this is a model and not the real type.** `PyroVec`/`PyroBuf` keep their
//! `ref_count` as an `AtomicU32` living *inside a raw allocation* at a negative
//! offset from the message pointer. Loom's atomics are not plain integers —
//! they carry per-object exploration state and cannot be materialized from a
//! pointer into raw memory. So this file reproduces the protocol
//! `src/io/buf.rs` implements, orderings included, and explores it exhaustively:
//!
//!   * `PyroBuf::view` transfers its count (no increment),
//!   * `PyroView::clone` is `fetch_add(1, Relaxed)`,
//!   * `PyroView::drop` / `PyroBuf::drop` is `fetch_sub(1, AcqRel)`, releasing
//!     iff the previous value was 1,
//!   * a null vtable means borrowed, so nothing is released.
//!
//! If the orderings in `buf.rs` change, they must change here too — that
//! coupling is the price of modelling, and is cheaper than making the real
//! layout loom-aware.
//!
//! What it asserts: **exactly one release**, for both pool-owned and self-owned
//! buffers, under every interleaving of clone and drop across threads; and that
//! no release happens while any handle is still alive.
#![cfg(feature = "debug")]

use loom::sync::Arc;
use loom::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

/// The refcount plus a release counter standing in for the vtable call.
struct Shared {
    ref_count: AtomicU32,
    releases: AtomicUsize,
    /// Null vtable equivalent: a borrowed buffer cannot be released.
    releasable: bool,
}

impl Shared {
    fn sealed() -> Arc<Self> {
        Arc::new(Self {
            ref_count: AtomicU32::new(1), // seal() starts the count at one
            releases: AtomicUsize::new(0),
            releasable: true,
        })
    }

    fn borrowed() -> Arc<Self> {
        Arc::new(Self {
            ref_count: AtomicU32::new(1),
            releases: AtomicUsize::new(0),
            releasable: false,
        })
    }
}

/// Mirrors `PyroView::clone`.
fn acquire(s: &Shared) {
    s.ref_count.fetch_add(1, Ordering::Relaxed);
}

/// Mirrors `release_one` in `src/io/buf.rs`.
fn release(s: &Shared) {
    if s.ref_count.fetch_sub(1, Ordering::AcqRel) != 1 {
        return;
    }
    if !s.releasable {
        return;
    }
    s.releases.fetch_add(1, Ordering::Relaxed);
}

#[test]
fn two_threads_cloning_and_dropping_release_exactly_once() {
    loom::model(|| {
        let s = Shared::sealed();

        // One extra handle per thread, taken before either thread starts, so
        // loom is free to order the two releases either way.
        acquire(&s);
        acquire(&s);
        // Count is now 3: the original plus two handles.

        let a = Arc::clone(&s);
        let t1 = loom::thread::spawn(move || release(&a));
        let b = Arc::clone(&s);
        let t2 = loom::thread::spawn(move || release(&b));

        // The original handle (the one seal() created) drops last here, but
        // loom explores every ordering of the three.
        release(&s);

        t1.join().unwrap();
        t2.join().unwrap();

        assert_eq!(
            s.ref_count.load(Ordering::Acquire),
            0,
            "count did not reach zero"
        );
        assert_eq!(
            s.releases.load(Ordering::Acquire),
            1,
            "released {} times, expected exactly one",
            s.releases.load(Ordering::Acquire)
        );
    });
}

#[test]
fn a_clone_racing_a_drop_never_releases_early() {
    loom::model(|| {
        let s = Shared::sealed();
        acquire(&s); // two handles

        let a = Arc::clone(&s);
        let t = loom::thread::spawn(move || {
            // This thread clones then drops both: the count must never touch
            // zero while it holds a handle.
            acquire(&a);
            assert_eq!(
                a.releases.load(Ordering::Acquire),
                0,
                "released while a handle was still alive"
            );
            release(&a);
            release(&a);
        });

        release(&s);
        t.join().unwrap();

        assert_eq!(s.ref_count.load(Ordering::Acquire), 0);
        assert_eq!(s.releases.load(Ordering::Acquire), 1);
    });
}

#[test]
fn a_borrowed_view_is_never_released() {
    loom::model(|| {
        // make_view: no prefix behind it, so a null vtable must mean "cannot be
        // released" rather than "release through garbage".
        let s = Shared::borrowed();
        acquire(&s);

        let a = Arc::clone(&s);
        let t = loom::thread::spawn(move || release(&a));
        release(&s);
        t.join().unwrap();

        assert_eq!(s.ref_count.load(Ordering::Acquire), 0);
        assert_eq!(
            s.releases.load(Ordering::Acquire),
            0,
            "a borrowed buffer must never be released"
        );
    });
}

#[test]
fn a_pool_owned_buffer_returns_exactly_once() {
    // Identical protocol, different releaser: the pool is the `ctx`, and its
    // free-list push happens exactly once. Modelled separately because the
    // pooled path is the one where a double release corrupts a free list rather
    // than merely double-freeing.
    loom::model(|| {
        let s = Shared::sealed();
        acquire(&s);
        acquire(&s);

        let handles: Vec<_> = (0..2)
            .map(|_| {
                let h = Arc::clone(&s);
                loom::thread::spawn(move || release(&h))
            })
            .collect();
        release(&s);
        for h in handles {
            h.join().unwrap();
        }

        assert_eq!(
            s.releases.load(Ordering::Acquire),
            1,
            "a pooled chunk was returned more than once"
        );
    });
}
