//! The reader's `K` slots: pool-owned [`PyroBuf`]s reused row after row.
//!
//! Each slot is a sealed `PyroBuf` the pool keeps for good. A read takes a
//! free one, sizes it, and [lends](PyroBuf::lend) it out as a [`PyroView`]:
//! the view takes over the buffer's count, so its clones share it, and when
//! the last one drops — on whichever thread — the release vtable reclaims the
//! buffer and puts the slot back on the free list, waking any read waiting
//! for one.
//!
//! Every lent view holds an `Arc<Shared>` through the release `ctx`, so the
//! slots outlive the reader for as long as any row from it is alive.

use std::alloc::{self, Layout};
use std::cell::UnsafeCell;
use std::ffi::c_void;
use std::future::poll_fn;
use std::ptr::{self, NonNull};
use std::sync::{Arc, Mutex, MutexGuard, Weak};
use std::task::{Poll, Waker};

use pyroformat::buf::{
    ALIGN, FLAG_POOL_OWNED, HEADER_SIZE, MAX_PAYLOAD, PREFIX_SIZE, PyroBuf, PyroReleaseVTable,
    PyroView,
};

pub(crate) struct Pool {
    shared: Arc<Shared>,
}

struct Shared {
    slots: Box<[Slot]>,
    state: Mutex<State>,
}

/// Which slots are free, and who is waiting for one.
struct State {
    free: Vec<usize>,
    waiters: Vec<Waker>,
}

/// One pool-owned buffer; `None` until first use. The release `ctx` of its
/// buffer points here.
struct Slot {
    index: usize,
    /// The pool this slot belongs to, for the release to find its way back.
    home: Weak<Shared>,
    buf: UnsafeCell<Option<PyroBuf>>,
}

// SAFETY: a slot's buffer is only touched by whoever holds the slot: `acquire`
// between taking it off the free list and lending it, then the release that
// fires when its last view drops. The free list hands each slot to one holder.
unsafe impl Sync for Slot {}

fn slot_layout(cap: usize) -> Layout {
    Layout::from_size_align(PREFIX_SIZE + HEADER_SIZE + cap, ALIGN).expect("slot layout")
}

impl Pool {
    pub fn new(slots: usize) -> Self {
        assert!(slots > 0, "a WAL reader needs at least one slot");
        let shared = Arc::new_cyclic(|home| Shared {
            slots: (0..slots)
                .map(|index| Slot {
                    index,
                    home: home.clone(),
                    buf: UnsafeCell::new(None),
                })
                .collect(),
            state: Mutex::new(State {
                free: (0..slots).rev().collect(),
                waiters: Vec::new(),
            }),
        });
        Self { shared }
    }

    pub fn slots(&self) -> usize {
        self.shared.slots.len()
    }

    pub fn free(&self) -> usize {
        self.shared.lock().free.len()
    }

    /// Wait for a free slot, size it for at least `cap` payload bytes, and lend
    /// it out. The view is the slot's only handle; its last drop frees it.
    pub async fn acquire(&self, cap: usize) -> PyroView {
        let index = poll_fn(|cx| {
            let mut state = self.shared.lock();
            match state.free.pop() {
                Some(index) => Poll::Ready(index),
                None => {
                    if !state.waiters.iter().any(|w| w.will_wake(cx.waker())) {
                        state.waiters.push(cx.waker().clone());
                    }
                    Poll::Pending
                }
            }
        })
        .await;
        // SAFETY: just taken off the free list, so this is the slot's holder.
        let buf = unsafe { self.shared.buf(index, cap) };
        // The view's release gives back this strong count (`release_slot`).
        std::mem::forget(self.shared.clone());
        // SAFETY: the buffer is not touched again until `release_slot`
        // reclaims it.
        unsafe { buf.lend() }
    }
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, State> {
        // A panic while holding the lock leaves the state consistent: every
        // critical section is a push, a pop, or a take.
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Slot `index`'s buffer, holding at least `cap` payload bytes.
    ///
    /// # Safety
    /// The caller holds the slot, and the buffer is not lent.
    #[allow(clippy::mut_from_ref)]
    unsafe fn buf(&self, index: usize, cap: usize) -> &mut PyroBuf {
        let slot = &self.slots[index];
        // SAFETY: the caller holds the slot, so nobody else is looking at it.
        let buf = unsafe { &mut *slot.buf.get() };
        if buf.as_ref().is_none_or(|b| b.capacity() < cap) {
            if let Some(old) = buf.take() {
                // SAFETY: not lent, so nothing references it.
                unsafe { free_buf(old) };
            }
            *buf = Some(new_buf(slot, cap));
        }
        buf.as_mut().unwrap()
    }

    /// The last view of slot `index` dropped: take its buffer back and free it.
    fn give_back(&self, index: usize) {
        // SAFETY: the release fired, so this is the slot's holder and the lend
        // is over.
        unsafe { (*self.slots[index].buf.get()).as_mut().unwrap().reclaim() };
        let waiters = {
            let mut state = self.lock();
            state.free.push(index);
            std::mem::take(&mut state.waiters)
        };
        // Wake every waiter: one may have been dropped, and the rest re-register.
        for waker in waiters {
            waker.wake();
        }
    }
}

impl Drop for Shared {
    fn drop(&mut self) {
        // Every lent view holds a strong count, so every buffer is back.
        for slot in self.slots.iter_mut() {
            if let Some(buf) = slot.buf.get_mut().take() {
                // SAFETY: not lent, and `Shared` is going away.
                unsafe { free_buf(buf) };
            }
        }
    }
}

/// A sealed, pool-owned buffer of at least `cap` payload bytes for `slot`.
fn new_buf(slot: &Slot, cap: usize) -> PyroBuf {
    let cap = cap.next_multiple_of(ALIGN).max(ALIGN);
    assert!(cap <= MAX_PAYLOAD, "slot capacity overflow");
    let layout = slot_layout(cap);
    // SAFETY: the layout is nonzero-sized.
    let base = unsafe { alloc::alloc(layout) };
    if base.is_null() {
        alloc::handle_alloc_error(layout);
    }
    // SAFETY: `msg` is PREFIX_SIZE into a fresh, 16-aligned allocation of
    // exactly PREFIX_SIZE + HEADER_SIZE + cap bytes (`slot_layout`). The header
    // is zeroed so it is never read uninitialized; a read overwrites it. The
    // vtable is static, and its release hands the buffer back to `slot`, which
    // lives in the `Shared` the lend kept alive; `free_buf` frees it.
    unsafe {
        let msg = base.add(PREFIX_SIZE);
        ptr::write_bytes(msg, 0, HEADER_SIZE);
        PyroBuf::from_raw_parts(
            NonNull::new_unchecked(msg),
            cap as u32,
            0,
            FLAG_POOL_OWNED,
            slot as *const Slot as *mut c_void,
            &SLOT_RELEASE_VTABLE,
        )
    }
}

/// Free a buffer made by [`new_buf`]. Its release would hand it back to the
/// pool, so it is deallocated here instead of dropped.
///
/// # Safety
/// The buffer is not lent.
unsafe fn free_buf(buf: PyroBuf) {
    let msg = buf.as_ptr().as_ptr().cast_mut();
    let layout = slot_layout(buf.capacity());
    std::mem::forget(buf);
    // SAFETY: allocated in `new_buf` with exactly this layout, and nothing
    // references it.
    unsafe { alloc::dealloc(msg.sub(PREFIX_SIZE), layout) };
}

/// `ctx` is the buffer's [`Slot`], inside a `Shared` kept alive by the strong
/// count `acquire` leaked for this lend.
unsafe extern "C" fn release_slot(ctx: *mut c_void, _msg: *mut u8, _cap: u32) {
    // SAFETY: the leaked count keeps `Shared`, and so the slot, alive.
    let slot = unsafe { &*(ctx as *const Slot) };
    // SAFETY: `acquire` leaked exactly one strong count for this lend, and the
    // release fires once per lend.
    let shared = unsafe { Arc::from_raw(slot.home.as_ptr()) };
    shared.give_back(slot.index);
}

static SLOT_RELEASE_VTABLE: PyroReleaseVTable = PyroReleaseVTable {
    release: release_slot,
};
