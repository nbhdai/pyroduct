use std::alloc::{self, Layout};
use std::hash::Hasher;
use std::ops::{Deref, DerefMut};
use std::ptr::{self, NonNull};
use std::sync::atomic::{AtomicU32, Ordering};
use std::{fmt, io, slice};

use crate::format::ParseError;
use crate::format::header::{
    OwnedPyroData, PyroData, PyroHeader, PyroHeaderMut, PyroParser, UNIT_BYTES, UNIT_HEADER,
};
use crate::{CapturedError, PyroError};

// ============================================================================
// Function pointer types for FFI-safe memory management
// ============================================================================

type PyroVecDropper = unsafe extern "C" fn(ptr: *mut u8, capacity: u32);
type PyroVecGrower =
    unsafe extern "C" fn(ptr: *mut u8, old_capacity: u32, new_capacity: u32) -> *mut u8;

// ============================================================================
// PyroInner — shared state for PyroVec, PyroBuf
// ============================================================================

pub const INNER_HEADER: usize = 16;

// align(16) guarantees that the start of the `data` field naturally falls on a
// 16-byte boundary, which perfectly suits the PyroParser requirement.
#[repr(C, align(16))]
#[derive(Debug)]
struct PyroInner {
    pub ref_count: AtomicU32, // 4 bytes
    pub capacity: u32,        // 4 bytes

    // Explicit padding ensures the header totals exactly 16 bytes.
    // The next field (`data`) starts perfectly on a 16-byte boundary.
    _padding: u64, // 8 bytes

    // Starts at offset 16
    pub data: [u8; 0],
}

impl PyroInner {
    fn new(capacity: u32) -> Self {
        Self {
            ref_count: AtomicU32::new(0),
            capacity,
            _padding: 0,
            data: [],
        }
    }

    #[inline]
    pub fn data_ptr(&self) -> *mut u8 {
        self.data.as_ptr() as *mut u8
    }
}

// ============================================================================
// PyroVec
// ============================================================================

/// A 16-byte aligned buffer with a self-describing header.
/// Compatible with FFI passing as a raw pointer or TCP/Unix framing.
///
/// This is wholely owned with it's own reference counting.
pub struct PyroVec {
    view: NonNull<PyroInner>,
    dropper: PyroVecDropper,
    grower: PyroVecGrower,
}

// ============================================================================
// PyroVec trait impls
// ============================================================================

impl PyroData for PyroVec {
    #[inline]
    fn header(&self) -> &[u8; 16] {
        let inner = unsafe { self.view.as_ref() };
        unsafe { &*(inner.data_ptr() as *const [u8; 16]) }
    }
    fn capacity(&self) -> usize {
        let cap = unsafe { self.view.as_ref().capacity } as usize;
        cap - PyroParser::HEADER_SIZE
    }

    fn py_ref(&self) -> PyroRef<'_> {
        PyroRef {
            data: self.as_raw_slice(),
        }
    }

    fn py_ptr(&self) -> PyroRefPtr {
        PyroRefPtr::new(self.as_raw_slice().as_ptr())
    }
}

unsafe impl Send for PyroVec {}
unsafe impl Sync for PyroVec {}

impl OwnedPyroData for PyroVec {
    #[inline]
    fn header_mut(&mut self) -> &mut [u8; 16] {
        let inner = unsafe { self.view.as_ref() };
        unsafe { &mut *(inner.data_ptr() as *mut [u8; 16]) }
    }
}

impl Drop for PyroVec {
    fn drop(&mut self) {
        unsafe {
            let inner = self.view.as_ref();
            let capacity = inner.capacity;
            let ptr = self.view.as_ptr() as *mut u8;
            (self.dropper)(ptr, capacity);
        }
    }
}

impl PyroVec {
    /// Helper to dynamically compute the total layout of PyroInner + contiguous data
    fn layout_for_capacity(capacity: usize) -> Layout {
        let inner_layout = Layout::new::<PyroInner>();
        let data_layout = Layout::from_size_align(capacity, PyroParser::ALIGN).unwrap();
        let (layout, _) = inner_layout.extend(data_layout).unwrap();
        layout.pad_to_align()
    }

    extern "C" fn default_dropper(ptr: *mut u8, capacity: u32) {
        unsafe {
            let layout = Self::layout_for_capacity(capacity as usize);
            alloc::dealloc(ptr, layout);
        }
    }

    unsafe extern "C" fn default_grower(
        ptr: *mut u8,
        old_capacity: u32,
        new_capacity: u32,
    ) -> *mut u8 {
        unsafe {
            let old_layout = Self::layout_for_capacity(old_capacity as usize);
            let new_layout = Self::layout_for_capacity(new_capacity as usize);
            let new_ptr = alloc::realloc(ptr, old_layout, new_layout.size());
            if new_ptr.is_null() {
                alloc::handle_alloc_error(new_layout);
            }
            new_ptr
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let capacity = capacity + PyroParser::HEADER_SIZE;
        let layout = Self::layout_for_capacity(capacity);

        let raw_ptr = unsafe {
            let raw = alloc::alloc(layout);
            if raw.is_null() {
                alloc::handle_alloc_error(layout);
            }

            // Initialize the header block
            // Use write_unaligned to avoid Stacked Borrows issues:
            // the allocation layout (inner + data) is larger than PyroInner,
            // and ptr::write would create a tag only covering PyroInner's size.
            let inner_ptr = raw as *mut PyroInner;
            ptr::write_unaligned(inner_ptr, PyroInner::new(capacity as u32));

            // Write UNIT_BYTES to the start of the data segment
            let data_ptr = (inner_ptr as *mut u8).add(INNER_HEADER);
            ptr::copy_nonoverlapping(UNIT_BYTES.as_ptr(), data_ptr, UNIT_BYTES.len());
            inner_ptr
        };

        Self {
            view: unsafe { NonNull::new_unchecked(raw_ptr) },
            dropper: Self::default_dropper,
            grower: Self::default_grower,
        }
    }

    pub fn data_ptr(&self) -> *const u8 {
        let inner = unsafe { self.view.as_ref() };
        unsafe { inner.data_ptr().add(PyroParser::HEADER_SIZE) }
    }

    pub fn raw_ptr(&self) -> *const u8 {
        let inner = unsafe { self.view.as_ref() };
        inner.data_ptr()
    }

    pub fn as_raw_slice(&self) -> &[u8] {
        unsafe {
            slice::from_raw_parts(
                self.raw_ptr(),
                PyroParser::HEADER_SIZE + self.header_len() as usize,
            )
        }
    }

    pub fn as_raw_slice_mut(&mut self) -> &mut [u8] {
        unsafe {
            slice::from_raw_parts_mut(
                self.raw_ptr() as *mut u8,
                PyroParser::HEADER_SIZE + self.header_len() as usize,
            )
        }
    }

    pub fn push(&mut self, byte: u8) {
        let inner_len = self.header_len() as usize;
        if inner_len == self.capacity() {
            self.grow(1);
        }
        unsafe {
            let data_ptr = self.data_ptr() as *mut u8;
            ptr::write(data_ptr.add(inner_len), byte);
            self.set_len(inner_len as u32 + 1);
        }
    }

    pub fn extend_from_slice(&mut self, other: &[u8]) {
        let required = other.len();
        let inner = unsafe { self.view.as_ref() };
        let current_len = self.header_len() as usize;
        let current_cap = inner.capacity as usize;

        if current_len + required + PyroParser::HEADER_SIZE > current_cap {
            self.grow(required);
        }
        unsafe {
            let inner_mut = self.view.as_ptr();
            ptr::copy_nonoverlapping(
                other.as_ptr(),
                (*inner_mut)
                    .data_ptr()
                    .add(PyroParser::HEADER_SIZE + current_len),
                required,
            );
            self.set_len((current_len + required) as u32);
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.data_ptr(), self.header_len() as usize) }
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut(self.data_ptr() as *mut u8, self.header_len() as usize) }
    }

    pub fn clear(&mut self) {
        unsafe {
            self.set_len(0);
        }
    }

    pub(crate) fn grow(&mut self, additional: usize) {
        let inner = unsafe { self.view.as_ref() };
        let current_cap = inner.capacity;
        let current_len = self.header_len();

        let required_cap = current_len
            .checked_add(additional as u32)
            .and_then(|c| c.checked_add(PyroParser::HEADER_SIZE as u32))
            .expect("capacity overflow");

        let mut new_cap = current_cap.saturating_mul(2).max(required_cap);

        let remainder = new_cap as usize % PyroParser::ALIGN;
        if remainder != 0 {
            new_cap = new_cap
                .checked_add((PyroParser::ALIGN - remainder) as u32)
                .expect("capacity overflow during alignment");
        }

        unsafe {
            // Treat the entire struct as the opaque pointer to resize
            let old_ptr = self.view.as_ptr() as *mut u8;
            let new_raw = (self.grower)(old_ptr, current_cap, new_cap) as *mut PyroInner;

            self.view = NonNull::new_unchecked(new_raw);
            (*new_raw).capacity = new_cap; // Update the contiguous capacity field directly
        }
    }

    pub fn ok() -> Self {
        let vec = Self::with_capacity(0);
        unsafe {
            let inner_data_ptr = vec.view.as_ref().data_ptr();
            ptr::copy_nonoverlapping(
                UNIT_HEADER.0.as_ptr(),
                inner_data_ptr,
                PyroParser::HEADER_SIZE,
            );
        }
        vec
    }

    pub fn clone_from_pyro<T: PyroData>(source: &T) -> Self {
        let mut new_vec = Self::with_capacity(source.len());
        new_vec.extend_from_slice(&*source);
        new_vec.header_mut().copy_from_slice(source.header());
        new_vec
    }
}

impl Clone for PyroVec {
    fn clone(&self) -> Self {
        let mut new_vec = Self::with_capacity(self.len());
        new_vec.extend_from_slice(self.as_slice());
        new_vec.set_status_u8(self.status_u8());
        new_vec.set_wire_format(self.wire_format());
        new_vec.set_fn_id(self.fn_id());
        new_vec.set_class_id(self.class_id());
        new_vec.set_mux_id(self.mux_id());
        new_vec
    }
}

impl fmt::Debug for PyroVec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PyroVec")
            .field("len", &self.len())
            .field("capacity", &self.capacity())
            .field("status", &self.status())
            .field("wire_fmt", &self.wire_format())
            .field("class_id", &self.class_id())
            .field("fn_id", &self.fn_id())
            .field("mux_id", &self.mux_id())
            .field("data", &self.as_slice())
            .finish()
    }
}

impl PartialEq for PyroVec {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl Eq for PyroVec {}

impl std::hash::Hash for PyroVec {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_slice().hash(state);
    }
}

impl Deref for PyroVec {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl DerefMut for PyroVec {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}

impl io::Write for PyroVec {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

// ============================================================================
// PyroView
// ============================================================================

/// A temporary, zero-copy view into a PyroVec residing in a byte slice
/// (e.g., WASM memory or a memory-mapped file).
///
/// Note: This view holds a raw pointer and does not track the lifetime
/// of the underlying memory. The caller must ensure the memory remains valid.
pub struct PyroView {
    pub(super) ref_count: *const AtomicU32,
    pub(super) data: *const u8, // Needs to always be 16 byte aligned.
    pub(super) dropper: PyroVecDropper,
}

/// Borrows a view across an FFI boundary. This will
#[repr(C)]
pub struct PyroViewPtr {
    ref_count: *const AtomicU32,
    data: *const u8, // Needs to always be 16 byte aligned.
    dropper: PyroVecDropper,
}

unsafe impl Send for PyroViewPtr {}

impl Clone for PyroView {
    fn clone(&self) -> Self {
        unsafe {
            (*self.ref_count).fetch_add(1, Ordering::Relaxed);
        }
        Self {
            ref_count: self.ref_count,
            data: self.data,
            dropper: self.dropper,
        }
    }
}

impl Drop for PyroView {
    fn drop(&mut self) {
        unsafe {
            // Decrement the reference count when the view is dropped
            if (*self.ref_count).fetch_sub(1, Ordering::AcqRel) == 1 {
                // We were the last owner, drop the underlying memory
                let inner = self.ref_count as *mut PyroInner;
                let capacity = (*inner).capacity;
                (self.dropper)(inner as *mut u8, capacity);
            }
        }
    }
}

unsafe impl Send for PyroView {}
unsafe impl Sync for PyroView {}

impl PyroData for PyroView {
    #[inline]
    fn header(&self) -> &[u8; 16] {
        unsafe { &*(self.data as *const [u8; 16]) }
    }

    fn capacity(&self) -> usize {
        self.len()
    }

    fn py_ref(&self) -> PyroRef<'_> {
        PyroRef {
            data: self.as_raw_slice(),
        }
    }

    fn py_ptr(&self) -> PyroRefPtr {
        PyroRefPtr::new(self.as_raw_slice().as_ptr())
    }
}

impl PyroView {
    /// Reconstructs a `PyroView` from a raw `PyroViewPtr`.
    ///
    /// # Safety
    /// * `raw.ptr` must be non-null and properly aligned (16-byte alignment).
    /// * The memory referenced must remain valid for the duration this view is used.
    pub unsafe fn from_ptr(raw: PyroViewPtr) -> Result<Self, PyroError> {
        if let Err(parse_error) = unsafe { PyroParser::check_raw(raw.data) } {
            tracing::error!(?parse_error, "Checks failed for an FFI PyroViewPtr");
            let error = CapturedError::new(format!(
                "CRITICAL ERROR: Unable to construct a Ffi view due to {}",
                parse_error
            ))
            .with_location(std::panic::Location::caller())
            .with_backtrace(std::backtrace::Backtrace::force_capture());
            return Err(PyroError::HeaderFfi(error.into()));
        };

        let data = raw.data;
        if raw.ref_count.is_null() {
            return Err(PyroError::null_pointer());
        }
        let ref_count = raw.ref_count;
        unsafe {
            (*ref_count).fetch_add(1, Ordering::Relaxed);
        }
        Ok(Self {
            data,
            ref_count,
            dropper: raw.dropper,
        })
    }

    pub fn as_slice(&self) -> &[u8] {
        unsafe {
            slice::from_raw_parts(
                self.data.add(PyroParser::HEADER_SIZE),
                self.header_len() as usize,
            )
        }
    }

    pub fn as_raw_slice(&self) -> &[u8] {
        unsafe {
            slice::from_raw_parts(
                self.data,
                PyroParser::HEADER_SIZE + self.header_len() as usize,
            )
        }
    }

    pub fn ptr(&self) -> PyroViewPtr {
        PyroViewPtr {
            ref_count: self.ref_count,
            data: self.data,
            dropper: self.dropper,
        }
    }

    pub fn clone_to_vec(&self) -> PyroVec {
        let mut vec = PyroVec::clone_from_pyro(self);
        vec.extend_from_slice(self.as_slice());
        vec
    }
}

impl From<PyroVec> for PyroView {
    fn from(value: PyroVec) -> Self {
        value.view()
    }
}

impl PyroVec {
    pub fn view(self) -> PyroView {
        unsafe {
            let inner = self.view.as_ref();
            let ref_count = (&inner.ref_count) as *const AtomicU32;

            // Increment reference count for the new view
            (*ref_count).fetch_add(1, Ordering::Relaxed);

            let data = inner.data_ptr();
            let dropper = self.dropper;

            std::mem::forget(self); // Transfer ownership to PyroView

            PyroView {
                ref_count,
                data,
                dropper,
            }
        }
    }
}

impl Deref for PyroView {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl fmt::Debug for PyroView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PyroView")
            .field("len", &self.len())
            .field("capacity", &self.capacity())
            .field("status", &self.status())
            .field("wire_fmt", &self.wire_format())
            .field("class_id", &self.class_id())
            .field("fn_id", &self.fn_id())
            .field("mux_id", &self.mux_id())
            .finish()
    }
}

impl From<PyroView> for PyroViewPtr {
    fn from(view: PyroView) -> Self {
        view.ptr()
    }
}

/// Creates a `PyroView` from a reference.
///
/// This performs bounds checks and header validation assuming the offset
/// points to a 16-byte `PyroInner` struct, followed immediately by the Data Header.
///
/// # Arguments
///
/// * `wasm_memory` - The entire available memory buffer.
/// * `offset` - The index into `wasm_memory` where the `PyroInner` struct begins.
///
/// SAFETY: The caller needs to own the memory and the counter for this. It can only drop once the counter is 0 again.
pub unsafe fn make_view(
    counter: &AtomicU32,
    reference: PyroRef<'_>,
    dropper: PyroVecDropper,
) -> Result<PyroView, PyroError> {
    counter.fetch_add(1, Ordering::Relaxed);
    // 6. Construct View
    Ok(PyroView {
        ref_count: counter as *const AtomicU32,
        data: reference.data.as_ptr(),
        dropper,
    })
}

/// A raw pointer to a Pyro data structure.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PyroRefPtr(pub(super) *const u8);

unsafe impl Send for PyroRefPtr {}
unsafe impl Sync for PyroRefPtr {}

impl PyroRefPtr {
    pub fn new(ptr: *const u8) -> Self {
        Self(ptr)
    }

    pub fn as_ptr(&self) -> *const u8 {
        self.0
    }

    /// Reconstructs a `PyroRef` from a raw pointer.
    ///
    /// # Safety
    /// * The pointer must be non-null and properly aligned (16-byte alignment).
    /// * The memory referenced must remain valid for the lifetime `'a`.
    /// * The memory must contain a valid Pyro header and payload as specified by the header.
    pub unsafe fn assume_ref<'a>(&self) -> PyroRef<'a> {
        let ptr = self.0;
        let header = unsafe { &*(ptr as *const [u8; 16]) };
        let payload_len = PyroHeader::header_len(header) as usize;
        let total_len = PyroParser::HEADER_SIZE + payload_len;

        PyroRef {
            data: unsafe { slice::from_raw_parts(ptr, total_len) },
        }
    }

    /// Reconstructs a `PyroRef` from a raw pointer.
    ///
    /// # Safety
    /// * The pointer must be non-null and properly aligned (16-byte alignment).
    /// * The memory referenced must remain valid for the lifetime `'a`.
    /// * The memory must contain a valid Pyro header and payload as specified by the header.
    pub unsafe fn try_ref<'a>(&self) -> Result<PyroRef<'a>, PyroError> {
        let ptr = self.0;
        let header = unsafe { &*(ptr as *const [u8; 16]) };

        let payload_len = PyroHeader::header_len(header) as usize;
        let total_len = PyroParser::HEADER_SIZE + payload_len;
        if let Err(parse_error) = PyroParser::check(header) {
            tracing::error!(?parse_error, "Checks failed for an FFI PyroViewPtr");
            let error = CapturedError::new(format!(
                "CRITICAL ERROR: Unable to construct a Ffi view due to {}",
                parse_error
            ))
            .with_location(std::panic::Location::caller())
            .with_backtrace(std::backtrace::Backtrace::force_capture());
            return Err(PyroError::HeaderFfi(error.into()));
        };
        Ok(PyroRef {
            data: unsafe { slice::from_raw_parts(ptr, total_len) },
        })
    }
}

/// A safe, lifetime-bound view into a Pyro data structure.
///
/// Unlike `PyroView`, this does not participate in reference counting and is
/// tied strictly to the lifetime of the borrowed slice. This makes it a zero-cost
/// abstraction for passing Pyro data down the call stack.
#[derive(Clone, Copy)]
pub struct PyroRef<'a> {
    pub(super) data: &'a [u8],
}

impl<'a> PyroData for PyroRef<'a> {
    #[inline]
    fn header(&self) -> &[u8; 16] {
        // Safety: The constructor guarantees the slice is at least 16 bytes.
        unsafe { &*(self.data.as_ptr() as *const [u8; 16]) }
    }

    fn capacity(&self) -> usize {
        self.data.len().saturating_sub(PyroParser::HEADER_SIZE)
    }

    fn py_ref(&self) -> PyroRef<'_> {
        self.clone()
    }

    fn py_ptr(&self) -> PyroRefPtr {
        PyroRefPtr::new(self.data.as_ptr())
    }
}

impl<'a> PyroRef<'a> {
    /// Returns a raw pointer to the start of the Pyro data (including header).
    pub fn as_ptr(&self) -> PyroRefPtr {
        PyroRefPtr::new(self.data.as_ptr())
    }

    /// Creates a `PyroRef` from a byte slice.

    ///
    /// The slice must contain the 16-byte header followed by the payload.
    pub fn try_from_slice(data: &'a [u8]) -> Result<Self, PyroError> {
        if data.len() < PyroParser::HEADER_SIZE {
            return Err(ParseError::SliceTooSmall.into());
        }

        // Validate the raw pointer and header fields
        if let Err(parse_error) = unsafe { PyroParser::check_raw(data.as_ptr()) } {
            tracing::error!(?parse_error, "Checks failed for PyroRef slice");
            let error = CapturedError::new(format!(
                "CRITICAL ERROR: Unable to construct a PyroRef due to {}",
                parse_error
            ))
            .with_location(std::panic::Location::caller())
            .with_backtrace(std::backtrace::Backtrace::force_capture());
            return Err(PyroError::HeaderFfi(error.into()));
        }

        // Validate total bounds match the claimed payload length
        let header = unsafe { &*(data.as_ptr() as *const [u8; 16]) };
        let payload_len = PyroHeader::header_len(header) as usize;
        let total_required = PyroParser::HEADER_SIZE + payload_len;

        if data.len() < total_required {
            return Err(ParseError::LengthExceedsCapacity.into());
        }

        Ok(Self {
            // Narrow the slice to exactly the header + payload to prevent trailing garbage
            data: &data[..total_required],
        })
    }

    pub fn as_raw_slice(&self) -> &'a [u8] {
        self.data
    }

    pub fn as_slice(&self) -> &'a [u8] {
        &self.data[PyroParser::HEADER_SIZE..PyroParser::HEADER_SIZE + self.header_len() as usize]
    }
}

impl<'a> Deref for PyroRef<'a> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<'a> fmt::Debug for PyroRef<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PyroRef")
            .field("len", &self.len())
            .field("capacity", &self.capacity())
            .field("status", &self.status())
            .field("wire_fmt", &self.wire_format())
            .field("class_id", &self.class_id())
            .field("fn_id", &self.fn_id())
            .field("mux_id", &self.mux_id())
            .finish()
    }
}

/// Creates a `PyroView` from a raw memory slice and an offset.
///
/// This performs bounds checks and header validation assuming the offset
/// points to a 16-byte `PyroInner` struct, followed immediately by the Data Header.
///
/// # Arguments
///
/// * `wasm_memory` - The entire available memory buffer.
/// * `offset` - The index into `wasm_memory` where the `PyroInner` struct begins.
pub fn get_ref(wasm_memory: &[u8], offset: usize) -> Result<PyroRef<'_>, PyroError> {
    // 1. We need at least 16 bytes for PyroInner + 16 bytes for the Data Header
    if wasm_memory.len() < offset + PyroParser::HEADER_SIZE {
        return Err(ParseError::SliceTooSmall.into());
    }
    if offset % 16 != 0 {
        return Err(ParseError::MisalignedPointer.into());
    }

    // 2. Map the pointer into the Wasm memory space safely

    if let Err(parse_error) = PyroParser::check(&wasm_memory[offset..]) {
        tracing::error!(?parse_error, "Checks failed for an FFI PyroViewPtr");
        let error = CapturedError::new(format!(
            "CRITICAL ERROR: Unable to construct a Ffi view due to {}",
            parse_error
        ))
        .with_location(std::panic::Location::caller())
        .with_backtrace(std::backtrace::Backtrace::force_capture());
        return Err(PyroError::HeaderFfi(error.into()));
    };

    // Safety: Checked that it's a valid header above
    let header: [u8; 16] = wasm_memory[offset..offset + 16]
        .try_into()
        .map_err(|_| ParseError::LengthExceedsCapacity)?;
    let payload_len = PyroHeader::header_len(&header) as usize;

    // 4. Validate total bounds
    let total_required = PyroParser::HEADER_SIZE + payload_len;
    if wasm_memory.len() - offset < total_required {
        return Err(ParseError::LengthExceedsCapacity.into());
    }
    // 6. Construct View
    Ok(PyroRef {
        data: &wasm_memory[offset..offset + total_required],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pyrovec_buffer_layout() {
        let mut vec = PyroVec::with_capacity(10);
        vec.extend_from_slice(b"TEST");

        let raw = vec.as_raw_slice();

        let mut expected = [0u8; 20];
        expected[0..4].copy_from_slice(&4u32.to_le_bytes()); // Length
        expected[8] = 1; // Wire format
        expected[16..20].copy_from_slice(b"TEST"); // Payload

        assert_eq!(raw, &expected, "PyroVec raw buffer layout mismatch");
    }

    #[test]
    fn test_pyro_ref_ptr() {
        let mut vec = PyroVec::with_capacity(10);
        vec.extend_from_slice(b"TEST");
        let reference = PyroRef::try_from_slice(vec.as_raw_slice()).unwrap();

        let ptr = reference.as_ptr();
        let reconstructed = unsafe { ptr.assume_ref() };

        assert_eq!(reconstructed.as_slice(), b"TEST");
        assert_eq!(reconstructed.len(), 4);
    }

    #[test]
    fn test_view_ref_counting_lifecycle() {
        let vec = PyroVec::with_capacity(10);
        let inner_ptr = vec.view.as_ptr() as *const AtomicU32;

        assert_eq!(
            unsafe { (*inner_ptr).load(Ordering::Acquire) },
            0,
            "Initial ref count should be 0"
        );

        // 1. Creation
        let view1 = vec.view();
        assert_eq!(
            unsafe { (*inner_ptr).load(Ordering::Acquire) },
            1,
            "Creating a view should increment to 1"
        );

        // 2. Cloning
        let view2 = view1.clone();
        assert_eq!(
            unsafe { (*inner_ptr).load(Ordering::Acquire) },
            2,
            "Cloning should increment to 2"
        );

        // 3. Dropping
        drop(view1);
        assert_eq!(
            unsafe { (*inner_ptr).load(Ordering::Acquire) },
            1,
            "Dropping a view should decrement to 1"
        );

        // Can't reference the pointer
        drop(view2);
    }
}
