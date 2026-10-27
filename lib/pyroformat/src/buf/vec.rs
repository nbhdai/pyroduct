//! [`PyroVec`] — the unsealed builder.

use std::alloc;
use std::fmt;
use std::io;
use std::ops::{Deref, DerefMut};
use std::ptr::{self, NonNull};
use std::slice;
use std::sync::atomic::Ordering;

use crate::buf::DataStatus;
use crate::buf::error::PyroError;
use crate::buf::header::{HasHeader, HasHeaderMut, PROTOCOL_VERSION, PyroHeader, PyroHeaderMut};
use crate::buf::layout::{
    ALIGN, FLAG_SEALED, HEADER_SIZE, MAX_PAYLOAD, PREFIX_SIZE, layout_for, round_up,
};
use crate::buf::prefix::{self, init_header, init_prefix};
use crate::buf::release::HOST_RELEASE_VTABLE;
use crate::buf::sealed::PyroBuf;

/// An **unsealed builder**: growable, self-owned, plain host heap.
///
/// `Vec<u8>` with an alignment guarantee and a reserved header region. It
/// carries **no allocator function pointers at all** — the vtable slot stays
/// null until [`seal`](Self::seal) fills in the host release vtable. Pluggable
/// release is a property of [`PyroBuf`], the only type a pool or foreign
/// allocator ever owns.
///
/// It exists for the two cases that cannot know their size in advance:
/// caller-built payloads, and the error path — which is what makes it
/// load-bearing rather than a convenience. An error must be reportable when the
/// pool is exhausted, so it cannot come from the pool.
///
/// By construction there is no `view()`, no FFI pointer accessor, and no
/// allocator vtable. Growth reallocates, which would dangle a view's refcount
/// pointer; the hazard is designed out rather than documented around.
pub struct PyroVec {
    /// Points at the message header, i.e. `base + PREFIX_SIZE`.
    msg: NonNull<u8>,
    /// **Payload** capacity. A copy of the prefix's `capacity`, kept in the
    /// handle so the write path's bounds checks read a register instead of
    /// the heap. The prefix stays authoritative for anything that only has
    /// the message pointer; [`Self::grow_to`] updates both.
    capacity: usize,
}

unsafe impl Send for PyroVec {}
unsafe impl Sync for PyroVec {}

impl PyroVec {
    /// Allocate room for `payload_cap` payload bytes.
    ///
    /// # Panics
    /// If `payload_cap` exceeds [`MAX_PAYLOAD`]. Use
    /// [`try_with_capacity`](Self::try_with_capacity) for the typed error.
    pub fn with_capacity(payload_cap: usize) -> Self {
        Self::try_with_capacity(payload_cap).expect("capacity overflow")
    }

    /// Allocate room for `payload_cap` payload bytes, reporting the 4 GiB limit
    /// as a typed error rather than truncating (fix (5) of §2).
    pub fn try_with_capacity(payload_cap: usize) -> Result<Self, PyroError> {
        if payload_cap > MAX_PAYLOAD {
            return Err(PyroError::MessageTooLarge(payload_cap));
        }
        let layout = layout_for(payload_cap);
        let base = unsafe { alloc::alloc(layout) };
        if base.is_null() {
            alloc::handle_alloc_error(layout);
        }
        let msg = unsafe { base.add(PREFIX_SIZE) };
        unsafe {
            init_prefix(msg, payload_cap as u32);
            init_header(msg);
        }
        Ok(Self {
            msg: unsafe { NonNull::new_unchecked(msg) },
            capacity: payload_cap,
        })
    }

    /// An empty payload with the unit header. Zero payload capacity, so this
    /// still allocates the 48-byte frame; [`PyroBuf::unit`] is the
    /// allocation-free form.
    /// A fresh buffer, `len` zeroed payload bytes — `vec![0u8; len]` for a
    /// `PyroVec`, and the common shape for pre-sizing a fixed region before
    /// a `PyroWriter::write` that indexes into it absolutely.
    pub fn zeroed(len: usize) -> Self {
        let mut v = Self::with_capacity(len);
        v.extend_zeroed(len);
        v
    }

    pub fn empty() -> Self {
        Self::with_capacity(0)
    }

    #[inline]
    fn msg_ptr(&self) -> *mut u8 {
        self.msg.as_ptr()
    }

    /// Payload bytes written so far. Stored in the header's `len` field, so
    /// there is one length, not two.
    #[inline]
    pub fn len(&self) -> usize {
        PyroHeader::payload_len(self) as usize
    }

    /// **Payload** capacity — the same meaning as everywhere else in this
    /// module (fix (2) of §2: the original subtracted `HEADER_SIZE` here and
    /// added it back in `extend_from_slice`).
    #[inline]
    pub fn capacity(&self) -> usize {
        debug_assert_eq!(self.capacity, unsafe { prefix::capacity(self.msg_ptr()) }
            as usize);
        self.capacity
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn payload(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.msg_ptr().add(HEADER_SIZE), self.len()) }
    }

    /// Mutable payload access. The borrow ends before any reallocation can
    /// happen, because `&mut self` is held for its duration — the borrow
    /// checker forbids a payload slice outliving it, which is the invariant the
    /// WASM story depends on (§4).
    #[inline]
    pub fn payload_mut(&mut self) -> &mut [u8] {
        let len = self.len();
        unsafe { slice::from_raw_parts_mut(self.msg_ptr().add(HEADER_SIZE), len) }
    }

    /// The whole message: header followed by payload.
    #[inline]
    pub fn as_message(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.msg_ptr(), HEADER_SIZE + self.len()) }
    }

    /// Ensure room for `additional` more payload bytes.
    pub fn reserve(&mut self, additional: usize) {
        let required = self
            .len()
            .checked_add(additional)
            .expect("capacity overflow");
        if required <= self.capacity() {
            return;
        }
        self.grow_to(required);
    }

    fn grow_to(&mut self, required: usize) {
        assert!(required <= MAX_PAYLOAD, "capacity overflow");
        let old_cap = self.capacity();
        // Amortized doubling, floored at the requirement, rounded to ALIGN so
        // the total allocation stays a multiple of 16.
        let new_cap = round_up(old_cap.saturating_mul(2).max(required).max(ALIGN)).min(MAX_PAYLOAD);
        assert!(new_cap >= required, "capacity overflow");

        let old_layout = layout_for(old_cap);
        let new_layout = layout_for(new_cap);
        let old_base = unsafe { self.msg_ptr().sub(PREFIX_SIZE) };
        let new_base = unsafe { alloc::realloc(old_base, old_layout, new_layout.size()) };
        if new_base.is_null() {
            alloc::handle_alloc_error(new_layout);
        }
        let new_msg = unsafe { new_base.add(PREFIX_SIZE) };
        unsafe { prefix::set_capacity(new_msg, new_cap as u32) };
        self.msg = unsafe { NonNull::new_unchecked(new_msg) };
        self.capacity = new_cap;
    }

    #[inline]
    fn set_len(&mut self, len: usize) {
        debug_assert!(len <= self.capacity());
        // Safe by the debug_assert plus the reserve that precedes every call.
        unsafe { PyroHeaderMut::set_payload_len(self, len as u32) };
    }

    pub fn push(&mut self, byte: u8) {
        self.reserve(1);
        let len = self.len();
        unsafe { ptr::write(self.msg_ptr().add(HEADER_SIZE + len), byte) };
        self.set_len(len + 1);
    }

    pub fn extend_from_slice(&mut self, bytes: &[u8]) {
        self.reserve(bytes.len());
        let len = self.len();
        unsafe {
            ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                self.msg_ptr().add(HEADER_SIZE + len),
                bytes.len(),
            );
        }
        self.set_len(len + bytes.len());
    }

    pub fn extend_zeroed(&mut self, n: usize) {
        self.reserve(n);
        let len = self.len();
        unsafe { ptr::write_bytes(self.msg_ptr().add(HEADER_SIZE + len), 0, n) };
        self.set_len(len + n);
    }

    /// Reset the payload length. Capacity and the allocation are untouched.
    pub fn clear(&mut self) {
        self.set_len(0);
    }

    /// `Vec<u8>::resize` parity: grows to `new_len` with `value`-filled bytes,
    /// or shrinks (dropping bytes past `new_len`, capacity untouched) — no
    /// reallocation on shrink, and on grow only if `new_len` exceeds capacity.
    pub fn resize(&mut self, new_len: usize, value: u8) {
        let len = self.len();
        if new_len > len {
            self.reserve(new_len - len);
            unsafe {
                ptr::write_bytes(self.msg_ptr().add(HEADER_SIZE + len), value, new_len - len)
            };
        }
        self.set_len(new_len);
    }

    /// O(1) re-tag into a sealed, self-owned message.
    ///
    /// Writes the schema hash and status into the header, freezes capacity,
    /// installs the host release vtable, and starts the refcount at one.
    /// Because [`PyroVec`] and [`PyroBuf`] share the layout, this is a re-tag,
    /// not a copy: same allocation, same pointer, zero allocations.
    pub fn seal(self, schema_hash: u64) -> PyroBuf {
        let msg = self.msg;
        unsafe {
            let p = msg.as_ptr();
            {
                // Scoped so this reference never outlives the header bytes it
                // covers, and is never used to reach past them.
                let header = &mut *(p as *mut [u8; HEADER_SIZE]);
                header.set_schema_hash(schema_hash);
                header.set_version(PROTOCOL_VERSION);
                header.clear_reserved();
            }
            prefix::set_ctx(p, ptr::null_mut());
            prefix::set_vtable(p, &HOST_RELEASE_VTABLE as *const _);
            prefix::set_local_flags(p, FLAG_SEALED);
            prefix::refcount(p).store(1, Ordering::Relaxed);
        }

        // Do not run our Drop: ownership moves to the PyroBuf unchanged.
        std::mem::forget(self);
        let mut buf = PyroBuf::from_sealed_msg(msg);
        buf.set_status(DataStatus::Data);
        buf
    }
}

impl Drop for PyroVec {
    fn drop(&mut self) {
        // Unsealed: self-owned, host heap, refcount never live. Free directly
        // rather than through the vtable, which is still null.
        unsafe {
            alloc::dealloc(self.msg_ptr().sub(PREFIX_SIZE), layout_for(self.capacity()));
        }
    }
}

impl HasHeader for PyroVec {
    #[inline]
    fn header_bytes(&self) -> &[u8; HEADER_SIZE] {
        unsafe { &*(self.msg_ptr() as *const [u8; HEADER_SIZE]) }
    }
}

impl HasHeaderMut for PyroVec {
    #[inline]
    fn header_bytes_mut(&mut self) -> &mut [u8; HEADER_SIZE] {
        unsafe { &mut *(self.msg_ptr() as *mut [u8; HEADER_SIZE]) }
    }
}

impl Deref for PyroVec {
    type Target = [u8];
    #[inline]
    fn deref(&self) -> &[u8] {
        self.payload()
    }
}

impl DerefMut for PyroVec {
    #[inline]
    fn deref_mut(&mut self) -> &mut [u8] {
        self.payload_mut()
    }
}

impl AsRef<[u8]> for PyroVec {
    fn as_ref(&self) -> &[u8] {
        self.payload()
    }
}

/// Payload-content equality against anything byte-slice-shaped (`PyroVec`,
/// `Vec<u8>`, `[u8]`, `[u8; N]`, ...) — capacity and any header state are not
/// part of this comparison. Chiefly for tests asserting two writers produced
/// the same bytes.
impl<T: AsRef<[u8]> + ?Sized> PartialEq<T> for PyroVec {
    fn eq(&self, other: &T) -> bool {
        self.payload() == other.as_ref()
    }
}

impl Eq for PyroVec {}

impl io::Write for PyroVec {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl fmt::Debug for PyroVec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.payload(), f)
    }
}
