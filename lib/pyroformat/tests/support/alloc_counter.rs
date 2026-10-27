//! An arming/disarming counting global allocator.
//!
//! Needed before anything can *prove* allocation-freedom or zero-copy sealing.
//! An assertion like "sealing a self-owned buffer performs zero allocations" is
//! only worth writing if something can actually observe an allocation.
//!
//! Counting is **per-thread**, via `const`-initialized thread-locals. Two
//! reasons: a global counter would pick up noise from the test harness's other
//! threads, and a lazily-initialized thread-local would allocate during its own
//! initialization and recurse. `try_with` is used throughout so a hook firing
//! during TLS teardown degrades to "not counted" rather than panicking inside
//! the allocator.
//!
//! Usage:
//! ```ignore
//! let g = AllocGuard::arm();
//! let buf = v.seal(hash, status);
//! assert_eq!(g.allocs(), 0, "sealing must not allocate");
//! ```

#![allow(dead_code)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

thread_local! {
    static ARMED: Cell<bool> = const { Cell::new(false) };
    static ALLOCS: Cell<usize> = const { Cell::new(0) };
    static DEALLOCS: Cell<usize> = const { Cell::new(0) };
    static BYTES: Cell<usize> = const { Cell::new(0) };
}

/// Forwards to the system allocator, counting calls made on armed threads.
pub struct CountingAlloc;

impl CountingAlloc {
    pub const fn new() -> Self {
        Self
    }
}

#[inline]
fn bump(cell: &'static std::thread::LocalKey<Cell<usize>>, by: usize) {
    let _ = cell.try_with(|c| c.set(c.get().wrapping_add(by)));
}

#[inline]
fn armed() -> bool {
    ARMED.try_with(|a| a.get()).unwrap_or(false)
}

unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if armed() {
            bump(&ALLOCS, 1);
            bump(&BYTES, layout.size());
        }
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if armed() {
            bump(&DEALLOCS, 1);
        }
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if armed() {
            // A realloc is one allocation event: it can move the buffer, which
            // is exactly what a "no reallocation" assertion is watching for.
            bump(&ALLOCS, 1);
            bump(&BYTES, new_size);
        }
        unsafe { System.realloc(ptr, layout, new_size) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        if armed() {
            bump(&ALLOCS, 1);
            bump(&BYTES, layout.size());
        }
        unsafe { System.alloc_zeroed(layout) }
    }
}

/// Arms counting for the current thread; disarms on drop.
///
/// Nesting is not supported — the inner guard's `Drop` would disarm early. One
/// guard per assertion, dropped before the next.
pub struct AllocGuard {
    _private: (),
}

impl AllocGuard {
    /// Zero the counters and start counting on this thread.
    pub fn arm() -> Self {
        ALLOCS.with(|c| c.set(0));
        DEALLOCS.with(|c| c.set(0));
        BYTES.with(|c| c.set(0));
        ARMED.with(|a| a.set(true));
        Self { _private: () }
    }

    pub fn allocs(&self) -> usize {
        ALLOCS.with(|c| c.get())
    }

    pub fn deallocs(&self) -> usize {
        DEALLOCS.with(|c| c.get())
    }

    pub fn bytes(&self) -> usize {
        BYTES.with(|c| c.get())
    }

    /// Stop counting early, keeping the counts readable.
    pub fn disarm(&self) {
        ARMED.with(|a| a.set(false));
    }
}

impl Drop for AllocGuard {
    fn drop(&mut self) {
        self.disarm();
    }
}
