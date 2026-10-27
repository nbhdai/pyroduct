//! [`PyroView`] / [`PyroViewPtr`] — refcounted handles, and [`make_view`] for
//! memory this process does not own.

use std::ffi::c_void;
use std::fmt;
use std::ops::Deref;
use std::ptr;
use std::slice;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::buf::error::PyroError;
use crate::buf::header::{HasHeader, PyroHeader, PyroParser};
use crate::buf::layout::HEADER_SIZE;
use crate::buf::prefix::release_one;
use crate::buf::reference::{PyroRef, PyroRefPtr};
use crate::buf::release::PyroReleaseVTable;

/// A refcounted handle to a sealed buffer.
///
/// Participates in the same refcount as its [`PyroBuf`], so a handler may keep
/// a reply buffer alive past the call that produced it without the pool
/// reclaiming it.
pub struct PyroView {
    ref_count: *const AtomicU32,
    data: *mut u8,
    ctx: *mut c_void,
    vtable: *const PyroReleaseVTable,
    /// Payload capacity. Redundant with the allocation for a prefixed buffer —
    /// the allocation is the single source of truth — but load-bearing for the
    /// **borrowed** case ([`make_view`], WASM linear memory) where no prefix
    /// exists to read it from. That is fix (3) of §2, kept for that reason and
    /// documented as such.
    capacity: u32,
}

unsafe impl Send for PyroView {}
unsafe impl Sync for PyroView {}

impl PyroView {
    /// Assemble a handle from an already-owned reference.
    ///
    /// Crate-internal: the caller must already hold the count this handle will
    /// release. `PyroBuf::view` transfers its own rather than incrementing, so
    /// there is exactly one release either way.
    ///
    /// # Safety
    /// `ref_count` is live for as long as this handle, `data` points at a valid
    /// message header, and `vtable` is either null (borrowed — nothing to
    /// release) or correct for this allocation.
    #[inline]
    pub(crate) unsafe fn from_parts(
        ref_count: *const AtomicU32,
        data: *mut u8,
        ctx: *mut c_void,
        vtable: *const PyroReleaseVTable,
        capacity: u32,
    ) -> Self {
        Self {
            ref_count,
            data,
            ctx,
            vtable,
            capacity,
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        PyroHeader::payload_len(self) as usize
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// **Payload** capacity — not `len()`, which is what the original returned.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity as usize
    }

    #[inline]
    pub fn payload(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.data.add(HEADER_SIZE), self.len()) }
    }

    #[inline]
    pub fn as_message(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.data, HEADER_SIZE + self.len()) }
    }

    #[inline]
    pub fn as_ref(&self) -> PyroRef<'_> {
        PyroRef {
            data: self.as_message(),
        }
    }

    #[inline]
    pub fn as_ptr(&self) -> PyroRefPtr {
        PyroRefPtr(self.data as *const u8)
    }

    #[inline]
    pub fn ref_count(&self) -> u32 {
        unsafe { (*self.ref_count).load(Ordering::Acquire) }
    }

    /// Transfer this handle's reference out across FFI. The count is *not*
    /// decremented; the receiver owns it.
    pub fn into_raw(self) -> PyroViewPtr {
        let raw = PyroViewPtr {
            ref_count: self.ref_count,
            data: self.data,
            ctx: self.ctx,
            vtable: self.vtable,
            capacity: self.capacity,
        };
        std::mem::forget(self);
        raw
    }

    /// Take back a reference handed out by [`into_raw`](Self::into_raw).
    ///
    /// # Safety
    /// `raw` must come from `into_raw` and be transferred exactly once. Taking
    /// it twice double-frees; never taking it leaks.
    pub unsafe fn from_raw(raw: PyroViewPtr) -> Result<Self, PyroError> {
        if raw.ref_count.is_null() {
            return Err(PyroError::NullPointer);
        }
        unsafe { PyroParser::check_raw(raw.data) }?;
        Ok(Self {
            ref_count: raw.ref_count,
            data: raw.data,
            ctx: raw.ctx,
            vtable: raw.vtable,
            capacity: raw.capacity,
        })
    }
}

impl Clone for PyroView {
    fn clone(&self) -> Self {
        // Relaxed is enough to *add* a reference: the new handle cannot observe
        // a release, and the AcqRel on the decrement side orders the teardown.
        unsafe { (*self.ref_count).fetch_add(1, Ordering::Relaxed) };
        Self {
            ref_count: self.ref_count,
            data: self.data,
            ctx: self.ctx,
            vtable: self.vtable,
            capacity: self.capacity,
        }
    }
}

impl Drop for PyroView {
    fn drop(&mut self) {
        unsafe {
            release_one(
                self.ref_count,
                self.ctx,
                self.vtable,
                self.data,
                self.capacity,
            )
        };
    }
}

impl HasHeader for PyroView {
    #[inline]
    fn header_bytes(&self) -> &[u8; HEADER_SIZE] {
        unsafe { &*(self.data as *const [u8; HEADER_SIZE]) }
    }
}

impl Deref for PyroView {
    type Target = [u8];
    #[inline]
    fn deref(&self) -> &[u8] {
        self.payload()
    }
}

impl fmt::Debug for PyroView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PyroView")
            .field("len", &self.len())
            .field("capacity", &self.capacity())
            .field("status", &self.status())
            .field("refs", &self.ref_count())
            .finish()
    }
}

/// The `#[repr(C)]` transfer form of a [`PyroView`].
///
/// Fat rather than a bare pointer because a *borrowed* buffer has no prefix to
/// read the refcount, vtable and capacity from.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PyroViewPtr {
    pub ref_count: *const AtomicU32,
    pub data: *mut u8,
    pub ctx: *mut c_void,
    pub vtable: *const PyroReleaseVTable,
    pub capacity: u32,
}

unsafe impl Send for PyroViewPtr {}
unsafe impl Sync for PyroViewPtr {}

/// Build a refcounted view over memory this process does not own — WASM linear
/// memory, a memory-mapped file. There is no prefix, so nothing can be
/// released through it; the caller's `counter` governs its lifetime.
///
/// # Safety
/// The caller owns both the memory and `counter`, and must keep the memory
/// valid until the counter returns to its prior value.
pub unsafe fn make_view(counter: &AtomicU32, reference: PyroRef<'_>) -> PyroView {
    counter.fetch_add(1, Ordering::Relaxed);
    PyroView {
        ref_count: counter as *const AtomicU32,
        data: reference.data.as_ptr() as *mut u8,
        ctx: ptr::null_mut(),
        // Null vtable: borrowed, cannot be released. Correct, and expressible.
        vtable: ptr::null(),
        capacity: reference.payload().len() as u32,
    }
}
