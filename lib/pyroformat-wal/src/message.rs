//! Owned buffers compio can borrow for an operation: sealed pyroformat
//! messages going out, and exact-length reads coming in — into scratch, or
//! into a pool-lent view.

use std::mem::MaybeUninit;

use compio::buf::{IoBuf, IoBufMut, SetLen};
use pyroformat::buf::{HEADER_SIZE, PyroView};

/// A `Vec<u8>` that a compio read fills with exactly `want` bytes. A plain
/// `Vec` exposes its whole capacity, which may be more than was asked for.
pub(crate) struct ExactBuf {
    vec: Vec<u8>,
    want: usize,
}

impl ExactBuf {
    pub fn new(want: usize) -> Self {
        Self {
            vec: Vec::with_capacity(want),
            want,
        }
    }

    pub fn into_vec(self) -> Vec<u8> {
        self.vec
    }
}

impl IoBuf for ExactBuf {
    fn as_init(&self) -> &[u8] {
        &self.vec
    }
}

impl SetLen for ExactBuf {
    unsafe fn set_len(&mut self, len: usize) {
        debug_assert!(len <= self.want);
        // SAFETY: the caller initialized `..len`, which is within capacity.
        unsafe { self.vec.set_len(len) }
    }
}

impl IoBufMut for ExactBuf {
    fn as_uninit(&mut self) -> &mut [MaybeUninit<u8>] {
        // SAFETY: the allocation holds at least `want` bytes and `&mut self`
        // borrows it uniquely; viewing initialized bytes as `MaybeUninit` is
        // always sound.
        unsafe { std::slice::from_raw_parts_mut(self.vec.as_mut_ptr().cast(), self.want) }
    }
}

/// A [`PyroView`] that a compio read fills with exactly `want` message bytes,
/// header first. Dropping it, even once the kernel is done with an abandoned
/// read, drops the view.
pub(crate) struct ViewFill {
    view: PyroView,
    want: usize,
    filled: usize,
}

// SAFETY: the view is `Send`, and the fill only writes through it.
unsafe impl Send for ViewFill {}

impl ViewFill {
    /// # Safety
    /// `view` is the only handle to its buffer, which this process owns and
    /// may write.
    ///
    /// # Panics
    /// If `want` exceeds the view's header and payload capacity.
    pub unsafe fn new(view: PyroView, want: usize) -> Self {
        assert!(want <= HEADER_SIZE + view.capacity(), "read past the view");
        debug_assert_eq!(view.ref_count(), 1, "filling a shared view");
        Self {
            view,
            want,
            filled: 0,
        }
    }

    /// The message bytes read so far.
    pub fn bytes(&self) -> &[u8] {
        self.as_init()
    }

    pub fn into_view(self) -> PyroView {
        self.view
    }

    fn msg(&self) -> *mut u8 {
        self.view.as_ptr().as_ptr().cast_mut()
    }
}

impl IoBuf for ViewFill {
    fn as_init(&self) -> &[u8] {
        // SAFETY: the first `filled` bytes were written by the read.
        unsafe { std::slice::from_raw_parts(self.msg(), self.filled) }
    }
}

impl SetLen for ViewFill {
    unsafe fn set_len(&mut self, len: usize) {
        debug_assert!(len <= self.want);
        self.filled = len;
    }
}

impl IoBufMut for ViewFill {
    fn as_uninit(&mut self) -> &mut [MaybeUninit<u8>] {
        // SAFETY: `want` bytes from the message pointer are within the
        // buffer (`new`), and the view is its only handle.
        unsafe { std::slice::from_raw_parts_mut(self.msg().cast(), self.want) }
    }
}
