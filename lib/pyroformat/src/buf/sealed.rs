//! [`PyroBuf`] — the sealed message.

use std::alloc;
use std::ffi::c_void;
use std::fmt;
use std::ops::Deref;
use std::ptr::{self, NonNull};
use std::slice;
use std::sync::atomic::Ordering;

use crate::buf::error::PyroError;
use crate::buf::header::{
    DataStatus, HasHeader, HasHeaderMut, ParseError, PyroHeader, PyroHeaderMut, PyroParser,
    UNIT_HEADER,
};
use crate::buf::layout::{
    FLAG_POOL_OWNED, FLAG_SEALED, HEADER_SIZE, MAX_PAYLOAD, PREFIX_SIZE, layout_for,
};
use crate::buf::prefix::{self, init_header, init_prefix, release_one};
use crate::buf::reference::{PyroRef, PyroRefPtr};
use crate::buf::release::{HOST_RELEASE_VTABLE, PyroReleaseVTable};
use crate::buf::view::PyroView;

/// A **sealed message**: fixed capacity, refcounted, FFI-safe.
///
/// Only a sealed buffer can produce a [`PyroView`] or cross FFI — enforced by
/// the type system, not by convention. Fixed capacity means a write that would
/// exceed it is [`PyroError::CapacityExceeded`]: a plain error, not a panic, not
/// a grow, not a silent truncation.
///
/// A `PyroBuf` is **exclusive**: whoever holds one is its only user, and
/// nothing here checks the refcount before a write. The pool guarantees it
/// when it hands a chunk to a writer, and the types keep it: the only ways to
/// share a buffer are [`view`](Self::view), which consumes the `PyroBuf`, and
/// sealing a [`PyroVec`](super::PyroVec), which starts it at one. Readers get
/// [`PyroView`]s, never a `PyroBuf`.
///
/// Ownership is recorded in `local_flags` and dispatched at refcount zero:
/// pool-owned chunks go back on their `pool_class` free list with `generation`
/// bumped; self-owned buffers are released through the vtable.
pub struct PyroBuf {
    msg: NonNull<u8>,
    /// **Payload** capacity. A copy of the prefix's `capacity`, which is
    /// frozen once sealed, kept in the handle so the write path's bounds
    /// checks read a register instead of the heap. The prefix stays
    /// authoritative for anything that only has the message pointer
    /// ([`PyroView`], the release vtable).
    capacity: usize,
    /// **Message** bytes, from the message pointer, known to be initialized:
    /// what the last I/O operation returned, or what the write path
    /// produced. Independent of the header's payload length — a read can
    /// return less than the header it brought in claims, and that gap is
    /// exactly what [`check_read`](Self::check_read) refuses. It is the
    /// extent compio sees through `IoBuf` (src/buf/compio.rs).
    init_cap: usize,
}

unsafe impl Send for PyroBuf {}
unsafe impl Sync for PyroBuf {}

impl PyroBuf {
    /// The process-wide unit message: `len == 0`, `schema_hash == 0`,
    /// `status == Empty`. No allocation, no refcount, no pool interaction, and
    /// the one exemption from the schema-hash check.
    #[inline]
    pub fn unit() -> PyroRef<'static> {
        PyroRef {
            data: &UNIT_HEADER.as_bytes()[..],
        }
    }

    /// Adopt a buffer that some other allocator owns.
    ///
    /// # Safety
    /// * `msg` points [`PREFIX_SIZE`] bytes into a live allocation of exactly
    ///   `layout_for(capacity)` bytes, 16-aligned at its base.
    /// * The prefix is writable and this call takes sole ownership of it.
    /// * `vtable` stays valid for as long as any handle to this buffer lives,
    ///   and its `release` correctly frees the allocation given the message
    ///   pointer and capacity.
    pub unsafe fn from_raw_parts(
        msg: NonNull<u8>,
        capacity: u32,
        pool_class: u16,
        local_flags: u16,
        ctx: *mut c_void,
        vtable: *const PyroReleaseVTable,
    ) -> Self {
        unsafe {
            let p = msg.as_ptr();
            prefix::set_ctx(p, ctx);
            prefix::set_vtable(p, vtable);
            prefix::set_capacity(p, capacity);
            prefix::set_pool_class(p, pool_class);
            prefix::set_local_flags(p, local_flags | FLAG_SEALED);
            prefix::refcount(p).store(1, Ordering::Relaxed);
        }
        let mut buf = Self {
            msg,
            capacity: capacity as usize,
            init_cap: 0,
        };
        buf.init_cap = HEADER_SIZE + buf.len();
        buf
    }

    /// A fresh, self-owned, sealed message of exactly `len` payload bytes,
    /// all zero, stamped [`DataStatus::Data`] with `schema_hash`.
    ///
    /// Capacity is `len` and so is the length: the whole payload is already
    /// claimed, for a writer that knows the final size up front and fills it
    /// in place. Zeroed because the fixed region's layout assumes it (a null
    /// scalar leaves its slot as the layout zeroed it); `alloc_zeroed` gets
    /// that for free on fresh pages rather than a separate `memset`.
    ///
    /// # Panics
    /// If `len` exceeds [`MAX_PAYLOAD`].
    #[inline]
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn new_valid(len: usize, schema_hash: u64) -> Self {
        Self::new_valid_in(len, schema_hash, true)
    }

    /// [`Self::new_valid`], but the payload is left as the allocator hands
    /// it over: whoever fills it writes every byte of the message itself
    /// (the header is still stamped). Saves `alloc_zeroed`'s memset, which
    /// on a large message costs more than writing it.
    #[inline]
    pub(crate) fn new_valid_uninit(len: usize, schema_hash: u64) -> Self {
        Self::new_valid_in(len, schema_hash, false)
    }

    #[inline(always)]
    fn new_valid_in(len: usize, schema_hash: u64, zeroed: bool) -> Self {
        assert!(len <= MAX_PAYLOAD, "capacity overflow");
        let layout = layout_for(len);
        let base = unsafe {
            if zeroed {
                alloc::alloc_zeroed(layout)
            } else {
                alloc::alloc(layout)
            }
        };
        if base.is_null() {
            alloc::handle_alloc_error(layout);
        }
        let msg = unsafe { NonNull::new_unchecked(base.add(PREFIX_SIZE)) };
        let mut buf = unsafe {
            let p = msg.as_ptr();
            init_prefix(p, len as u32);
            init_header(p);
            prefix::set_vtable(p, &HOST_RELEASE_VTABLE as *const _);
            prefix::set_local_flags(p, FLAG_SEALED);
            prefix::refcount(p).store(1, Ordering::Relaxed);
            Self::from_sealed_msg(msg)
        };
        unsafe { buf.set_len_unchecked(len) };
        buf.set_schema_hash(schema_hash);
        buf.set_status(DataStatus::Data);
        buf
    }

    /// Wrap a message pointer whose prefix is already fully stamped.
    ///
    /// Crate-internal, and the only way [`PyroVec::seal`](super::PyroVec::seal)
    /// can hand ownership over: `seal` writes the header and prefix in place,
    /// then re-tags. Keeping `msg` private means no other module can conjure a
    /// `PyroBuf` from an unsealed pointer.
    #[inline]
    pub(crate) fn from_sealed_msg(msg: NonNull<u8>) -> Self {
        let capacity = unsafe { prefix::capacity(msg.as_ptr()) } as usize;
        debug_assert!(
            unsafe { prefix::local_flags(msg.as_ptr()) } & FLAG_SEALED != 0,
            "from_sealed_msg called on a buffer that was never sealed"
        );
        let mut buf = Self {
            msg,
            capacity,
            init_cap: 0,
        };
        buf.init_cap = HEADER_SIZE + buf.len();
        buf
    }

    #[inline]
    pub(crate) fn msg_ptr(&self) -> *mut u8 {
        self.msg.as_ptr()
    }

    #[inline]
    pub fn header(&self) -> &[u8; HEADER_SIZE] {
        self.header_bytes()
    }

    #[inline]
    pub fn len(&self) -> usize {
        PyroHeader::payload_len(self) as usize
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// **Payload** capacity. Fixed once sealed, so the handle's copy never
    /// drifts from the prefix's.
    #[inline]
    pub fn capacity(&self) -> usize {
        debug_assert_eq!(self.capacity, unsafe { prefix::capacity(self.msg_ptr()) }
            as usize);
        self.capacity
    }

    /// Message bytes known to be initialized — see the field. Header plus
    /// payload after any write-path change; after a read, what the kernel
    /// returned.
    #[inline]
    pub fn init_cap(&self) -> usize {
        self.init_cap
    }

    /// Record `init_cap` message bytes as initialized. For I/O completions.
    ///
    /// # Safety
    /// `init_cap <= HEADER_SIZE + capacity()`, and the first `init_cap`
    /// message bytes are initialized.
    #[inline]
    pub(crate) unsafe fn set_init_cap(&mut self, init_cap: usize) {
        debug_assert!(init_cap <= HEADER_SIZE + self.capacity);
        self.init_cap = init_cap;
    }

    /// Validate a message an I/O read just brought in: the header must be a
    /// header, the payload it claims must fit this buffer, and the read must
    /// have returned all of it.
    ///
    /// * [`PyroError::Header`] — fewer than [`HEADER_SIZE`] bytes arrived
    ///   ([`SliceTooSmall`](crate::buf::ParseError::SliceTooSmall)), or they
    ///   are not a valid header (version, reserved bytes).
    /// * [`PyroError::CapacityExceeded`] — the header claims `need` payload
    ///   bytes, more than this buffer's `have`. Not a bad message: the pool
    ///   reallocates a buffer of at least `need` and retries the read. Never
    ///   grown here — capacity is fixed once sealed.
    /// * [`PyroError::Header`]`(`[`SliceTooSmall`](crate::buf::ParseError::SliceTooSmall)`)`
    ///   — the read returned fewer bytes than the header claims.
    ///
    /// Bytes past the claimed message are not an error; they stay counted in
    /// [`init_cap`](Self::init_cap).
    pub fn check_read(&self) -> Result<(), PyroError> {
        if self.init_cap < HEADER_SIZE {
            return Err(ParseError::SliceTooSmall.into());
        }
        // SAFETY: at least HEADER_SIZE initialized bytes at the message pointer.
        unsafe { PyroParser::check_raw(self.msg_ptr()) }?;
        let need = self.len();
        if need > self.capacity {
            return Err(PyroError::CapacityExceeded {
                need,
                have: self.capacity,
            });
        }
        if self.init_cap < HEADER_SIZE + need {
            return Err(ParseError::SliceTooSmall.into());
        }
        Ok(())
    }

    #[inline]
    pub fn payload(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.msg_ptr().add(HEADER_SIZE), self.len()) }
    }

    /// The whole message: header followed by payload. This is what goes on the
    /// socket.
    #[inline]
    pub fn as_message(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.msg_ptr(), HEADER_SIZE + self.len()) }
    }

    /// The size class this chunk came from, or 0 for a self-owned buffer. The
    /// pool learns it without a side table.
    #[inline]
    pub fn pool_class(&self) -> u16 {
        unsafe { prefix::pool_class(self.msg_ptr()) }
    }

    #[inline]
    pub fn local_flags(&self) -> u16 {
        unsafe { prefix::local_flags(self.msg_ptr()) }
    }

    #[inline]
    pub fn is_pool_owned(&self) -> bool {
        self.local_flags() & FLAG_POOL_OWNED != 0
    }

    /// The domain this chunk was issued by, packed into `local_flags` (§4.1
    /// of [docs/io-plan.md](../../docs/io-plan.md)). Zero for a self-owned,
    /// non-pooled buffer.
    #[inline]
    pub fn domain_id(&self) -> u16 {
        crate::buf::layout::unpack_domain_id(self.local_flags())
    }

    /// The pool's return counter (§8.2). Bumped when a pool-owned chunk goes
    /// back on its free list; it is an ABA tag for the free list and backs the
    /// pool's double-return assertion. It is **not** a stale-pointer check on
    /// [`PyroRefPtr`] — see §8.2 for why that cannot be built as specified.
    #[inline]
    pub fn generation(&self) -> u32 {
        unsafe { prefix::generation(self.msg_ptr()) }
    }

    /// Bump the return counter. For the pool's use at reclaim time.
    ///
    /// # Safety
    /// Only the owner reclaiming this chunk may call this, and only while no
    /// other handle to it exists.
    #[inline]
    pub unsafe fn bump_generation(&mut self) -> u32 {
        unsafe {
            let g = prefix::generation(self.msg_ptr()).wrapping_add(1);
            prefix::set_generation(self.msg_ptr(), g);
            g
        }
    }

    /// Current reference count. For assertions and pool bookkeeping.
    #[inline]
    pub fn ref_count(&self) -> u32 {
        unsafe { prefix::refcount(self.msg_ptr()).load(Ordering::Acquire) }
    }

    /// Mutable payload access. Used by receive paths that fill a buffer then
    /// re-stamp its header.
    #[inline]
    pub fn payload_mut(&mut self) -> &mut [u8] {
        let len = self.len();
        unsafe { slice::from_raw_parts_mut(self.msg_ptr().add(HEADER_SIZE), len) }
    }

    /// The full fixed capacity as a mutable slice. The receive path fills this
    /// and then sets the length.
    #[inline]
    pub fn capacity_mut(&mut self) -> &mut [u8] {
        let cap = self.capacity();
        unsafe { slice::from_raw_parts_mut(self.msg_ptr().add(HEADER_SIZE), cap) }
    }

    /// Set the payload length after filling the buffer. Fixed capacity, so a
    /// length past the end is [`PyroError::CapacityExceeded`] — never a grow and
    /// never a truncation.
    ///
    /// Safety: Need to make sure all bytes are initalized
    pub unsafe fn set_len(&mut self, len: usize) -> Result<(), PyroError> {
        let have = self.capacity();
        if len > have {
            return Err(PyroError::CapacityExceeded { need: len, have });
        }
        unsafe { self.set_len_unchecked(len) };
        Ok(())
    }

    /// Append to the payload within the fixed capacity.
    pub fn write_payload(&mut self, bytes: &[u8]) -> Result<(), PyroError> {
        let len = self.len();
        let have = self.capacity();
        let need = len + bytes.len();
        if need > have {
            return Err(PyroError::CapacityExceeded { need, have });
        }
        unsafe {
            ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                self.msg_ptr().add(HEADER_SIZE + len),
                bytes.len(),
            );
            self.set_len_unchecked(need);
        }
        Ok(())
    }

    /// Unchecked length set. The caller must have already bounds-checked
    /// `len <= capacity`. The write path produced the whole message, so all
    /// of it counts as initialized.
    #[inline]
    pub(crate) unsafe fn set_len_unchecked(&mut self, len: usize) {
        unsafe { PyroHeaderMut::set_payload_len(self, len as u32) };
        self.init_cap = HEADER_SIZE + len;
    }

    #[inline]
    pub fn set_status(&mut self, s: DataStatus) {
        PyroHeaderMut::set_status(self, s);
    }

    #[inline]
    pub fn set_schema_hash(&mut self, h: u64) {
        PyroHeaderMut::set_schema_hash(self, h);
    }

    #[inline]
    pub fn set_flags(&mut self, f: u8) {
        PyroHeaderMut::set_flags(self, f);
    }

    #[inline]
    pub fn set_trace_id(&mut self, trace_id: u128) {
        PyroHeaderMut::set_trace_id(self, trace_id);
    }

    #[inline]
    pub fn set_parent_span(&mut self, span: u64) {
        PyroHeaderMut::set_parent_span(self, span);
    }

    #[inline]
    pub fn set_sampled(&mut self, sampled: bool) {
        PyroHeaderMut::set_sampled(self, sampled);
    }

    /// Nanoseconds since the Unix epoch.
    #[inline]
    pub fn set_timestamp(&mut self, t: i64) {
        PyroHeaderMut::set_timestamp(self, t);
    }

    /// Turn into a refcounted handle. Only sealed buffers can produce one.
    ///
    /// Consuming, so the count this `PyroBuf` held transfers rather than being
    /// incremented — there is exactly one release either way.
    pub fn view(mut self) -> PyroView {
        // SAFETY: `self` is forgotten, so it is never used or dropped again.
        let view = unsafe { self.lend() };
        std::mem::forget(self);
        view
    }

    /// Hand out a view while keeping this handle — for a pool that owns its
    /// chunks for good and lends them out one message after another.
    ///
    /// Like [`view`](Self::view), the view takes over this handle's count
    /// rather than adding one, so the release fires when the last view drops.
    /// For a lent chunk that release is the pool's signal that it is free.
    ///
    /// # Safety
    /// From here until that release fires this `PyroBuf` holds no count: it
    /// must not be used, lent again, or dropped. After the release, call
    /// [`reclaim`](Self::reclaim) before doing any of those.
    pub unsafe fn lend(&mut self) -> PyroView {
        let msg = self.msg.as_ptr();
        unsafe {
            PyroView::from_parts(
                prefix::refcount_ptr(msg),
                msg,
                prefix::ctx(msg),
                prefix::vtable(msg),
                prefix::capacity(msg),
            )
        }
    }

    /// Take back a [lent](Self::lend) chunk once its last view has dropped:
    /// restore the count it lent out, so this handle is a whole `PyroBuf`
    /// again.
    ///
    /// # Safety
    /// This buffer was lent, and the release for that lend has fired.
    pub unsafe fn reclaim(&mut self) {
        let refcount = unsafe { prefix::refcount(self.msg_ptr()) };
        debug_assert_eq!(
            refcount.load(Ordering::Relaxed),
            0,
            "reclaimed a live buffer"
        );
        refcount.store(1, Ordering::Relaxed);
    }

    /// A borrowed, zero-cost handle.
    #[inline]
    pub fn as_ref(&self) -> PyroRef<'_> {
        PyroRef {
            data: self.as_message(),
        }
    }

    /// The `#[repr(C)]` raw pointer that crosses FFI.
    #[inline]
    pub fn as_ptr(&self) -> PyroRefPtr {
        PyroRefPtr(self.msg_ptr() as *const u8)
    }
}

impl Drop for PyroBuf {
    fn drop(&mut self) {
        // A PyroBuf always owns its prefix, so the release triple comes from
        // there. Read before the decrement, while the allocation is certainly
        // still alive.
        unsafe {
            let msg = self.msg_ptr();
            let refcount = prefix::refcount_ptr(msg);
            let ctx = prefix::ctx(msg);
            let vtable = prefix::vtable(msg);
            let cap = prefix::capacity(msg);
            release_one(refcount, ctx, vtable, msg, cap);
        }
    }
}

impl HasHeader for PyroBuf {
    #[inline]
    fn header_bytes(&self) -> &[u8; HEADER_SIZE] {
        unsafe { &*(self.msg_ptr() as *const [u8; HEADER_SIZE]) }
    }
}

impl HasHeaderMut for PyroBuf {
    #[inline]
    fn header_bytes_mut(&mut self) -> &mut [u8; HEADER_SIZE] {
        unsafe { &mut *(self.msg_ptr() as *mut [u8; HEADER_SIZE]) }
    }
}

impl Deref for PyroBuf {
    type Target = [u8];
    #[inline]
    fn deref(&self) -> &[u8] {
        self.payload()
    }
}

impl fmt::Debug for PyroBuf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PyroBuf")
            .field("len", &self.len())
            .field("capacity", &self.capacity())
            .field("init_cap", &self.init_cap)
            .field("status", &self.status())
            .field("schema_hash", &format_args!("{:#018x}", self.schema_hash()))
            .field("refs", &self.ref_count())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buf::PyroVec;

    #[test]
    fn new_valid_matches_a_sealed_zeroed_vec() {
        for len in [0usize, 1, 17, 4096] {
            let buf = PyroBuf::new_valid(len, 0xDEAD_BEEF_CAFE_F00D);
            let expected = PyroVec::zeroed(len).seal(0xDEAD_BEEF_CAFE_F00D);
            assert_eq!(buf.len(), len);
            assert_eq!(buf.capacity(), len);
            assert_eq!(buf.ref_count(), 1);
            assert_eq!(buf.local_flags(), FLAG_SEALED);
            assert_eq!(buf.as_message(), expected.as_message(), "len {len}");
        }
    }

    #[test]
    fn lend_releases_at_the_last_view_and_reclaim_restores_the_count() {
        use std::sync::atomic::AtomicU32;

        unsafe extern "C" fn count_release(ctx: *mut c_void, _msg: *mut u8, _cap: u32) {
            unsafe { &*(ctx as *const AtomicU32) }.fetch_add(1, Ordering::Relaxed);
        }
        static COUNT_RELEASE: PyroReleaseVTable = PyroReleaseVTable {
            release: count_release,
        };

        let released = AtomicU32::new(0);
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
                &released as *const AtomicU32 as *mut c_void,
                &COUNT_RELEASE,
            )
        };
        for round in 1..=2 {
            let view = unsafe { buf.lend() };
            assert_eq!(view.ref_count(), 1, "the view took over the count");
            let clone = view.clone();
            drop(view);
            assert_eq!(released.load(Ordering::Relaxed), round - 1);
            drop(clone);
            assert_eq!(
                released.load(Ordering::Relaxed),
                round,
                "last view released"
            );
            unsafe { buf.reclaim() };
            assert_eq!(buf.ref_count(), 1);
        }
        // The release vtable only counts, so free the allocation by hand.
        std::mem::forget(buf);
        unsafe { alloc::dealloc(base, layout) };
    }
}
