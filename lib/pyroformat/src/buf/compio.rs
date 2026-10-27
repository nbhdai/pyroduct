//! compio's buffer traits for [`PyroBuf`] and [`PyroView`], behind the
//! `compio` feature.
//!
//! The bytes compio hands the kernel are the **message**: the header, then
//! the payload, starting at the message pointer. The 32-byte local prefix
//! behind it (release vtable, `ref_count`, capacity, ...) is wire-invisible
//! and stays that way: every slice here starts *after* it, so no write ever
//! ships the refcount and no read can overwrite it.
//!
//! **Extents** of a [`PyroBuf`] come from out-of-band state, never from the
//! header — a read overwrites the header, so its `payload_len` cannot say how
//! far the read got:
//!
//! * [`IoBuf::as_init`] is the first [`init_cap`](PyroBuf::init_cap)
//!   message bytes. The write path keeps that equal to header + payload, so
//!   a write sends exactly [`PyroBuf::as_message`].
//! * [`IoBufMut::as_uninit`] is `HEADER_SIZE + capacity()`, the capacity
//!   frozen at seal and held in the handle.
//! * [`SetLen::set_len`] records what the kernel returned in `init_cap`,
//!   leaving the payload length to the header that arrived.
//!
//! **Reading.** compio reads into `as_uninit()` from offset 0 and then calls
//! `advance_to(n)`, which only ever grows `init_cap`. Clear it first —
//! [`SetLen::clear`] — so `init_cap` ends up as exactly what this read
//! returned, then [`PyroBuf::check_read`] the result:
//!
//! ```ignore
//! SetLen::clear(&mut buf);
//! let BufResult(res, buf) = file.read_at(buf, pos).await;
//! res?;
//! match buf.check_read() {
//!     Ok(()) => { /* a whole message */ }
//!     // The pool reallocates a buffer of at least `need` and retries.
//!     Err(PyroError::CapacityExceeded { need, .. }) => { /* ... */ }
//!     // A short read, or not a header.
//!     Err(e) => return Err(e),
//! }
//! ```
//!
//! **Lifetime** follows the refcount, not the operation. compio takes the
//! handle by value and keeps it until the kernel completes, even if the
//! future is dropped first; the handle's count keeps the allocation (and a
//! pooled chunk's place off its free list) alive for that whole time. When
//! compio finally drops it, [`release_one`](super::prefix::release_one) runs
//! as for any other drop — `AcqRel` decrement, vtable release at zero — so a
//! pool-owned chunk returns to its pool and a self-owned one is freed, never
//! while I/O is in flight.
//!
//! A [`PyroView`] is shared and read-only, so it is `IoBuf` only.

use std::mem::MaybeUninit;
use std::slice;

use compio_buf::{IoBuf, IoBufMut, SetLen};

use crate::buf::{HEADER_SIZE, PyroView, sealed::PyroBuf};

impl IoBuf for PyroBuf {
    /// The first `init_cap` message bytes: header and payload after any
    /// write-path change, what the last read returned after one.
    #[inline]
    fn as_init(&self) -> &[u8] {
        // SAFETY: `init_cap <= HEADER_SIZE + capacity`, the message region of
        // this live allocation, and those bytes are initialized — both are
        // the field's invariant.
        unsafe { slice::from_raw_parts(self.msg_ptr(), self.init_cap()) }
    }
}

impl IoBufMut for PyroBuf {
    /// The whole message region, `HEADER_SIZE + capacity()`, header first.
    #[inline]
    fn as_uninit(&mut self) -> &mut [MaybeUninit<u8>] {
        // SAFETY: the message region of this allocation, borrowed uniquely
        // through `&mut self`. Viewing initialized bytes as `MaybeUninit` is
        // always sound, and the prefix lies before the pointer, out of reach.
        unsafe { slice::from_raw_parts_mut(self.msg_ptr().cast(), HEADER_SIZE + self.capacity()) }
    }
}

impl SetLen for PyroBuf {
    /// Record `len` message bytes as returned by the I/O operation. Thex
    /// header's payload length is left as the read brought it in.
    #[inline]
    unsafe fn set_len(&mut self, len: usize) {
        // SAFETY: forwarded — `len` is within `as_uninit()` and initialized.
        unsafe { self.set_init_cap(len) }
    }
}

impl IoBuf for PyroView {
    /// Header and payload: `HEADER_SIZE + len()` bytes from the message
    /// pointer. Every byte is initialized — the header is stamped at seal,
    /// the payload up to `len` was written.
    #[inline]
    fn as_init(&self) -> &[u8] {
        self.as_message()
    }
}

#[cfg(test)]
mod tests {
    use std::alloc;
    use std::ffi::c_void;
    use std::mem::MaybeUninit;
    use std::ptr::NonNull;
    use std::sync::atomic::{AtomicU32, Ordering};

    use compio_buf::{IntoInner, IoBuf, IoBufMut, SetLen};

    use crate::buf::layout::{FLAG_POOL_OWNED, PREFIX_SIZE, layout_for};
    use crate::buf::prefix::init_header;
    use crate::buf::{HEADER_SIZE, ParseError, PyroBuf, PyroError, PyroReleaseVTable, PyroVec};

    fn message(payload: &[u8]) -> PyroBuf {
        let mut vec = PyroVec::with_capacity(payload.len());
        vec.extend_from_slice(payload);
        vec.seal(0xDEAD_BEEF_CAFE_F00D)
    }

    /// What the driver does: the kernel writes `bytes` at the start of
    /// `as_uninit()`, then the op calls `advance_to` with the count.
    fn kernel_read(buf: &mut PyroBuf, bytes: &[u8]) {
        let dst = IoBufMut::as_uninit(buf);
        for (d, s) in dst.iter_mut().zip(bytes) {
            *d = MaybeUninit::new(*s);
        }
        unsafe { SetLen::advance_to(buf, bytes.len()) };
    }

    /// A buffer handed to a read: `cap` payload bytes, `init_cap` cleared.
    fn read_target(cap: usize) -> PyroBuf {
        let mut buf = PyroBuf::new_valid(cap, 0);
        SetLen::clear(&mut buf);
        assert_eq!(buf.init_cap(), 0);
        buf
    }

    #[test]
    fn io_bytes_are_header_then_payload_never_the_prefix() {
        let buf = message(b"hello pyro");

        assert_eq!(IoBuf::as_init(&buf), buf.as_message());
        assert_eq!(IoBuf::buf_len(&buf), HEADER_SIZE + 10);
        assert_eq!(
            IoBuf::buf_ptr(&buf),
            buf.as_ptr().0,
            "starts at the message pointer"
        );
        assert_eq!(&IoBuf::as_init(&buf)[HEADER_SIZE..], b"hello pyro");
        assert!(
            !IoBuf::is_empty(&buf),
            "an empty payload still ships its header"
        );
    }

    #[test]
    fn write_path_keeps_init_cap_on_the_message() {
        let mut buf = PyroBuf::new_valid(32, 0);
        unsafe { buf.set_len(0).unwrap() };
        assert_eq!(IoBuf::as_init(&buf), buf.as_message());
        buf.write_payload(b"abc").unwrap();
        assert_eq!(IoBuf::as_init(&buf), buf.as_message());
        assert_eq!(buf.init_cap(), HEADER_SIZE + 3);
    }

    #[test]
    fn mutable_extent_is_header_plus_capacity() {
        let mut buf = PyroBuf::new_valid(40, 0);
        assert_eq!(IoBufMut::buf_capacity(&mut buf), HEADER_SIZE + 40);
        assert_eq!(
            IoBufMut::buf_mut_ptr(&mut buf).cast_const().cast(),
            buf.as_ptr().0
        );
    }

    #[test]
    fn a_whole_message_reads_in_and_checks() {
        let src = message(b"0123456789");
        let mut buf = read_target(64);
        kernel_read(&mut buf, src.as_message());

        assert_eq!(buf.init_cap(), HEADER_SIZE + 10);
        buf.check_read().unwrap();
        assert_eq!(buf.len(), 10, "length is the header's, not the read's");
        assert_eq!(buf.as_message(), src.as_message());
        assert_eq!(buf.capacity(), 64, "capacity untouched");
    }

    #[test]
    fn trailing_bytes_are_counted_but_not_an_error() {
        let src = message(b"0123456789");
        let mut bytes = src.as_message().to_vec();
        bytes.extend_from_slice(&[0xAA; 7]);
        let mut buf = read_target(64);
        kernel_read(&mut buf, &bytes);

        buf.check_read().unwrap();
        assert_eq!(buf.init_cap(), HEADER_SIZE + 17);
        assert_eq!(buf.payload(), b"0123456789");
    }

    #[test]
    fn a_short_read_is_an_error() {
        let src = message(b"0123456789");
        let mut buf = read_target(64);
        kernel_read(&mut buf, &src.as_message()[..HEADER_SIZE + 5]);

        assert!(matches!(
            buf.check_read(),
            Err(PyroError::Header(ParseError::SliceTooSmall))
        ));
    }

    #[test]
    fn a_partial_header_is_an_error() {
        let src = message(b"0123456789");
        let mut buf = read_target(64);
        kernel_read(&mut buf, &src.as_message()[..HEADER_SIZE - 1]);

        assert_eq!(buf.init_cap(), HEADER_SIZE - 1);
        assert!(matches!(
            buf.check_read(),
            Err(PyroError::Header(ParseError::SliceTooSmall))
        ));
    }

    #[test]
    fn a_header_claiming_past_capacity_asks_for_a_bigger_buffer() {
        let src = message(&[7u8; 100]);
        let mut buf = read_target(32);
        // The kernel fills as much as fits: the header and 32 payload bytes.
        kernel_read(&mut buf, &src.as_message()[..HEADER_SIZE + 32]);

        assert!(matches!(
            buf.check_read(),
            Err(PyroError::CapacityExceeded {
                need: 100,
                have: 32
            })
        ));
    }

    #[test]
    fn a_garbage_header_is_not_a_reallocation() {
        let mut bytes = [0u8; HEADER_SIZE];
        bytes[..4].copy_from_slice(&u32::MAX.to_le_bytes());
        let mut buf = read_target(32);
        kernel_read(&mut buf, &bytes);

        assert!(matches!(buf.check_read(), Err(PyroError::Header(_))));
    }

    #[test]
    fn slicing_takes_ownership_and_drops_through_the_refcount() {
        unsafe extern "C" fn count_release(ctx: *mut c_void, _msg: *mut u8, _cap: u32) {
            unsafe { &*(ctx as *const AtomicU32) }.fetch_add(1, Ordering::Relaxed);
        }
        static COUNT_RELEASE: PyroReleaseVTable = PyroReleaseVTable {
            release: count_release,
        };

        // Leaked so the counter outlives the `'static` buffer that points at it.
        let released: &'static AtomicU32 = Box::leak(Box::new(AtomicU32::new(0)));
        let layout = layout_for(16);
        let base = unsafe { alloc::alloc(layout) };
        let msg = unsafe { NonNull::new_unchecked(base.add(PREFIX_SIZE)) };
        unsafe { init_header(msg.as_ptr()) };
        let mut buf = unsafe {
            PyroBuf::from_raw_parts(
                msg,
                16,
                0,
                FLAG_POOL_OWNED,
                released as *const AtomicU32 as *mut c_void,
                &COUNT_RELEASE,
            )
        };
        buf.write_payload(b"0123456789").unwrap();

        // What a positioned write does with the message: own it, read a range.
        let slice = buf.slice(HEADER_SIZE..);
        assert_eq!(slice.as_init(), b"0123456789");
        assert_eq!(
            slice.as_inner().ref_count(),
            1,
            "compio holds the one count"
        );
        assert_eq!(released.load(Ordering::Relaxed), 0);

        let buf = slice.into_inner();
        assert_eq!(
            released.load(Ordering::Relaxed),
            0,
            "still owned after the op"
        );
        drop(buf);
        assert_eq!(
            released.load(Ordering::Relaxed),
            1,
            "released once, at zero"
        );

        // The release vtable only counts, so free the allocation by hand.
        unsafe { alloc::dealloc(base, layout) };
    }
}
