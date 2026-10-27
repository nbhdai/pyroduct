//! [`PyroRef`] — borrowed and lifetime-bound — and [`PyroRefPtr`], the bare
//! message pointer that is the FFI ABI.

use std::fmt;
use std::ops::Deref;
use std::slice;

use crate::buf::error::PyroError;
use crate::buf::header::{HasHeader, HasHeaderMut, PyroHeader, PyroParser};
use crate::buf::layout::HEADER_SIZE;
use crate::buf::vec::PyroVec;

/// A borrowed, lifetime-bound handle. Does not participate in refcounting, so
/// it is zero-cost for passing a message down the call stack. Crosses FFI
/// inward only — never outward, where the lifetime would be a lie.
#[derive(Clone, Copy)]
pub struct PyroRef<'a> {
    pub(crate) data: &'a [u8],
}

impl<'a> PyroRef<'a> {
    /// Validate a slice as a message: 16-aligned, valid header, and the claimed
    /// payload actually present.
    pub fn try_from_slice(bytes: &'a [u8]) -> Result<Self, PyroError> {
        PyroParser::check(bytes)?;
        let header: &[u8; HEADER_SIZE] = bytes[..HEADER_SIZE].try_into().unwrap();
        let total = HEADER_SIZE + PyroHeader::payload_len(header) as usize;
        Ok(Self {
            data: &bytes[..total],
        })
    }

    #[inline]
    pub fn header(&self) -> &'a [u8; HEADER_SIZE] {
        // The constructors guarantee at least HEADER_SIZE bytes.
        unsafe { &*(self.data.as_ptr() as *const [u8; HEADER_SIZE]) }
    }

    #[inline]
    pub fn payload(&self) -> &'a [u8] {
        &self.data[HEADER_SIZE..]
    }

    /// The whole message: header followed by payload.
    #[inline]
    pub fn as_message(&self) -> &'a [u8] {
        self.data
    }

    #[inline]
    pub fn as_ptr(&self) -> PyroRefPtr {
        PyroRefPtr(self.data.as_ptr())
    }

    /// Copy into a fresh growable buffer.
    pub fn to_vec(&self) -> PyroVec {
        let mut v = PyroVec::with_capacity(self.payload().len());
        v.extend_from_slice(self.payload());
        v.header_bytes_mut().copy_from_slice(self.header());
        v
    }
}

impl<'a> HasHeader for PyroRef<'a> {
    #[inline]
    fn header_bytes(&self) -> &[u8; HEADER_SIZE] {
        unsafe { &*(self.data.as_ptr() as *const [u8; HEADER_SIZE]) }
    }
}

impl<'a> Deref for PyroRef<'a> {
    type Target = [u8];
    #[inline]
    fn deref(&self) -> &[u8] {
        self.payload()
    }
}

impl<'a> fmt::Debug for PyroRef<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PyroRef")
            .field("len", &self.payload().len())
            .field("status", &self.status())
            .field("is_unit", &self.is_unit())
            .finish()
    }
}

/// A bare message pointer: 16-aligned and self-describing, so a callee needs no
/// out-parameter length and can run the zero-copy reader directly on it.
///
/// One word by design (§8.2): it carries no generation, because a bare message
/// pointer must stay self-sufficient and a tag a caller can forge would not
/// make retention sound anyway. Retention past return is governed by the FFI
/// safety contract, not by a check here.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PyroRefPtr(pub(crate) *const u8);

unsafe impl Send for PyroRefPtr {}
unsafe impl Sync for PyroRefPtr {}

impl PyroRefPtr {
    #[inline]
    pub fn new(ptr: *const u8) -> Self {
        Self(ptr)
    }

    #[inline]
    pub fn as_ptr(&self) -> *const u8 {
        self.0
    }

    #[inline]
    pub fn is_null(&self) -> bool {
        self.0.is_null()
    }

    /// Validate and borrow.
    ///
    /// # Safety
    /// The pointer must be non-null or null (null is reported, not read),
    /// 16-aligned, carry a valid header, and the payload must be readable for
    /// the header's `len` bytes for the lifetime `'a`.
    pub unsafe fn try_ref<'a>(&self) -> Result<PyroRef<'a>, PyroError> {
        unsafe { PyroParser::check_raw(self.0) }?;
        let header = unsafe { &*(self.0 as *const [u8; HEADER_SIZE]) };
        let total = HEADER_SIZE + PyroHeader::payload_len(header) as usize;
        Ok(PyroRef {
            data: unsafe { slice::from_raw_parts(self.0, total) },
        })
    }

    /// As [`try_ref`](Self::try_ref), plus the strict length check against a
    /// known capacity.
    ///
    /// # Safety
    /// As [`try_ref`](Self::try_ref).
    pub unsafe fn try_ref_strict<'a>(&self, capacity: usize) -> Result<PyroRef<'a>, PyroError> {
        unsafe { PyroParser::check_strict_raw(self.0, capacity) }?;
        unsafe { self.try_ref() }
    }

    /// Borrow without validating.
    ///
    /// # Safety
    /// As [`try_ref`](Self::try_ref), and the header must already be known good.
    pub unsafe fn assume_ref<'a>(&self) -> PyroRef<'a> {
        unsafe {
            let header = &*(self.0 as *const [u8; HEADER_SIZE]);
            let total = HEADER_SIZE + PyroHeader::payload_len(header) as usize;
            PyroRef {
                data: slice::from_raw_parts(self.0, total),
            }
        }
    }
}
