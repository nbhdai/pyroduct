//! The release seam: the one pluggable operation on a sealed buffer.

use std::alloc;
use std::ffi::c_void;

use crate::buf::layout::{PREFIX_SIZE, layout_for};

/// The one pluggable operation on a sealed buffer.
///
/// Allocation is deliberately *not* here: it happens at pool-fill time, where
/// it can be fallible and async without contorting a C signature. There is no
/// `grow` hook — all four of its problems (no context, no failure channel, no
/// granted-capacity report, a base pointer that moves under `memory.grow`) bite
/// mid-write, which is exactly when they are unfixable.
///
/// `ptr` is the **message** pointer, not the allocation base. A releaser that
/// owns the prefix subtracts [`PREFIX_SIZE`] itself; that keeps a bare message
/// pointer self-sufficient, which is the whole point of putting the vtable in
/// the prefix rather than in the handle.
#[repr(C)]
pub struct PyroReleaseVTable {
    /// Invoked when `ref_count` hits zero. Pool-owned buffers route here too,
    /// with the pool as `ctx`.
    pub release: unsafe extern "C" fn(ctx: *mut c_void, ptr: *mut u8, cap: u32),
}

unsafe extern "C" fn host_release(_ctx: *mut c_void, msg: *mut u8, cap: u32) {
    unsafe {
        let base = msg.sub(PREFIX_SIZE);
        alloc::dealloc(base, layout_for(cap as usize));
    }
}

/// Release through the host heap. Installed by [`PyroVec::seal`].
pub static HOST_RELEASE_VTABLE: PyroReleaseVTable = PyroReleaseVTable {
    release: host_release,
};
