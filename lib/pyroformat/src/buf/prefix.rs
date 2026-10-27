//! Raw accessors for the local prefix, addressed from the message pointer.
//!
//! Every one of these takes the message pointer and offsets backwards. None
//! creates a reference wider than the field it touches, and none is valid for a
//! *borrowed* buffer — a `PyroRef` into WASM guest memory has no prefix at all
//! (§4, §8.2).
//!
//! `release_one` lives here too: it is the single place the release decision is
//! made, shared by [`PyroBuf`](super::PyroBuf) and [`PyroView`](super::PyroView).

use std::ffi::c_void;
use std::ptr;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::buf::header::{HEADER_SIZE, PROTOCOL_VERSION, PyroParser};
use crate::buf::layout::{
    OFF_CAPACITY, OFF_CTX, OFF_GENERATION, OFF_LOCAL_FLAGS, OFF_POOL_CLASS, OFF_REFCOUNT,
    OFF_VTABLE,
};
use crate::buf::release::PyroReleaseVTable;

#[inline]
pub(crate) unsafe fn refcount<'a>(msg: *const u8) -> &'a AtomicU32 {
    unsafe { &*(msg.sub(OFF_REFCOUNT) as *const AtomicU32) }
}

#[inline]
pub(crate) unsafe fn refcount_ptr(msg: *const u8) -> *const AtomicU32 {
    unsafe { msg.sub(OFF_REFCOUNT) as *const AtomicU32 }
}

#[inline]
pub(crate) unsafe fn capacity(msg: *const u8) -> u32 {
    unsafe { ptr::read(msg.sub(OFF_CAPACITY) as *const u32) }
}

#[inline]
pub(crate) unsafe fn set_capacity(msg: *mut u8, cap: u32) {
    unsafe { ptr::write(msg.sub(OFF_CAPACITY) as *mut u32, cap) }
}

#[inline]
pub(crate) unsafe fn ctx(msg: *const u8) -> *mut c_void {
    unsafe { ptr::read(msg.sub(OFF_CTX) as *const *mut c_void) }
}

#[inline]
pub(crate) unsafe fn set_ctx(msg: *mut u8, ctx: *mut c_void) {
    unsafe { ptr::write(msg.sub(OFF_CTX) as *mut *mut c_void, ctx) }
}

#[inline]
pub(crate) unsafe fn vtable(msg: *const u8) -> *const PyroReleaseVTable {
    unsafe { ptr::read(msg.sub(OFF_VTABLE) as *const *const PyroReleaseVTable) }
}

#[inline]
pub(crate) unsafe fn set_vtable(msg: *mut u8, vt: *const PyroReleaseVTable) {
    unsafe { ptr::write(msg.sub(OFF_VTABLE) as *mut *const PyroReleaseVTable, vt) }
}

#[inline]
pub(crate) unsafe fn pool_class(msg: *const u8) -> u16 {
    unsafe { ptr::read(msg.sub(OFF_POOL_CLASS) as *const u16) }
}

#[inline]
pub(crate) unsafe fn set_pool_class(msg: *mut u8, c: u16) {
    unsafe { ptr::write(msg.sub(OFF_POOL_CLASS) as *mut u16, c) }
}

#[inline]
pub(crate) unsafe fn local_flags(msg: *const u8) -> u16 {
    unsafe { ptr::read(msg.sub(OFF_LOCAL_FLAGS) as *const u16) }
}

#[inline]
pub(crate) unsafe fn set_local_flags(msg: *mut u8, f: u16) {
    unsafe { ptr::write(msg.sub(OFF_LOCAL_FLAGS) as *mut u16, f) }
}

#[inline]
pub(crate) unsafe fn generation(msg: *const u8) -> u32 {
    unsafe { ptr::read(msg.sub(OFF_GENERATION) as *const u32) }
}

#[inline]
pub(crate) unsafe fn set_generation(msg: *mut u8, g: u32) {
    unsafe { ptr::write(msg.sub(OFF_GENERATION) as *mut u32, g) }
}

/// Write a freshly allocated prefix: no owner, no vtable, refcount zero.
///
/// Fields are written individually rather than by constructing a struct and
/// `ptr::write`-ing it. That is fix (4): a struct write creates a tag covering
/// only the struct's size, while every other access reaches past it from the
/// same pointer — precisely the pattern that trips Stacked and Tree Borrows.
pub(crate) unsafe fn init_prefix(msg: *mut u8, payload_cap: u32) {
    unsafe {
        set_ctx(msg, ptr::null_mut());
        set_vtable(msg, ptr::null());
        ptr::write(msg.sub(OFF_REFCOUNT) as *mut u32, 0u32);
        set_capacity(msg, payload_cap);
        set_pool_class(msg, 0);
        set_local_flags(msg, 0);
        set_generation(msg, 0);
    }
}

/// Stamp a fresh, unsealed message header: zeroed but for the version, so the
/// buffer parses from the moment it exists.
pub(crate) unsafe fn init_header(msg: *mut u8) {
    unsafe {
        ptr::write_bytes(msg, 0, HEADER_SIZE);
        ptr::write(msg.add(PyroParser::OFFSET_VERSION), PROTOCOL_VERSION);
    }
}

/// Drop one reference and release at zero. Shared by [`PyroBuf`] and
/// [`PyroView`], so there is exactly one place the release decision is made.
///
/// `ctx`/`vtable`/`cap` are passed in rather than read from the prefix. A
/// [`PyroView`] may be *borrowed* ([`make_view`]) — pointing into memory this
/// process does not own, which has no prefix behind it. Re-reading the prefix
/// here would then either free a buffer the view does not own, or read out of
/// bounds entirely. A null `vtable` is the borrowed case and means "cannot be
/// released", which is exactly what §4 says it should mean.
#[inline]
pub(crate) unsafe fn release_one(
    refcount: *const AtomicU32,
    ctx: *mut c_void,
    vtable: *const PyroReleaseVTable,
    msg: *mut u8,
    cap: u32,
) {
    unsafe {
        // AcqRel: everything written to the payload before the last release
        // must be visible to the releaser (and, for a pooled chunk, to the next
        // acquirer after the scrub).
        if (*refcount).fetch_sub(1, Ordering::AcqRel) != 1 {
            return;
        }
        if vtable.is_null() {
            return;
        }
        ((*vtable).release)(ctx, msg, cap);
    }
}
