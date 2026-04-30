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
use crate::{CapturedError, PyroError, PyroResult};

// ============================================================================
// Function pointer types for FFI-safe memory management
// ============================================================================

type PyroVecDropper = unsafe extern "C" fn(ptr: *mut u8, capacity: u32);
type PyroVecGrower =
    unsafe extern "C" fn(ptr: *mut u8, old_capacity: u32, new_capacity: u32) -> *mut u8;

// ============================================================================
// PyroInner — shared state for PyroVec, PyroBuf, and PyroView
// ============================================================================

pub const INNER_HEADER: usize = 16;

// align(16) guarantees that the start of the `data` field naturally falls on a
// 16-byte boundary, which perfectly suits the PyroParser requirement.
#[repr(C, align(16))]
#[derive(Debug)]
pub(crate) struct PyroInner {
    ref_count: AtomicU32, // 4 bytes
    capacity: u32,        // 4 bytes

    // Explicit padding ensures the header totals exactly 16 bytes.
    // The next field (`data`) starts perfectly on a 16-byte boundary.
    _padding: u64, // 4 bytes

    // Starts at offset 16
    data: [u8; 0],
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
pub struct PyroVec {
    pub(super) view: NonNull<PyroInner>,
    dropper: PyroVecDropper,
    grower: PyroVecGrower,
}

// ============================================================================
// PyroVecPtr — the FFI-safe representation
// ============================================================================

#[repr(C)]
pub struct PyroVecPtr {
    ptr: *mut PyroInner,
    dropper: PyroVecDropper,
    grower: PyroVecGrower,
}

unsafe impl std::marker::Send for PyroVecPtr {}

// ============================================================================
// PyroBuf — a PyroVec with the grow capability removed
// ============================================================================

pub struct PyroBuf {
    view: NonNull<PyroInner>,
    dropper: PyroVecDropper,
}

#[repr(C)]
pub struct PyroBufPtr {
    ptr: *mut PyroInner,
    dropper: PyroVecDropper,
}

// SAFETY: PyroBuf owns its allocation exclusively
unsafe impl Send for PyroBuf {}
unsafe impl Sync for PyroBuf {}

impl PyroData for PyroBuf {
    #[inline]
    fn header(&self) -> &[u8; 16] {
        let inner = unsafe { self.view.as_ref() };
        unsafe { &*(inner.data_ptr() as *const [u8; 16]) }
    }
    fn capacity(&self) -> usize {
        let cap = unsafe { self.view.as_ref().capacity } as usize;
        cap - PyroParser::HEADER_SIZE
    }
}

impl OwnedPyroData for PyroBuf {
    #[inline]
    fn header_mut(&mut self) -> &mut [u8; 16] {
        let inner = unsafe { self.view.as_ref() };
        unsafe { &mut *(inner.data_ptr() as *mut [u8; 16]) }
    }
}

impl PyroBuf {
    pub fn data_ptr(&self) -> *const u8 {
        let inner = unsafe { self.view.as_ref() };
        unsafe { inner.data_ptr().add(PyroParser::HEADER_SIZE) }
    }

    pub fn as_slice(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.data_ptr(), self.header_len() as usize) }
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut(self.data_ptr() as *mut u8, self.header_len() as usize) }
    }

    pub(crate) fn as_packet_slice(&self) -> &[u8] {
        unsafe {
            let inner = self.view.as_ref();
            slice::from_raw_parts(
                inner.data_ptr(),
                PyroParser::HEADER_SIZE + self.header_len() as usize,
            )
        }
    }

    pub(crate) fn as_packet_slice_mut(&mut self) -> &mut [u8] {
        unsafe {
            let inner = self.view.as_ref();
            slice::from_raw_parts_mut(
                inner.data_ptr(),
                PyroParser::HEADER_SIZE + self.header_len() as usize,
            )
        }
    }

    pub fn clear(&mut self) {
        unsafe {
            self.set_len(0);
        }
    }

    pub fn into_raw(self) -> PyroBufPtr {
        let raw = PyroBufPtr {
            ptr: self.view.as_ptr(),
            dropper: self.dropper,
        };
        std::mem::forget(self);
        raw
    }

    #[track_caller]
    pub unsafe fn from_raw(pointer: PyroBufPtr) -> PyroResult<Self> {
        let ptr = pointer.ptr;
        let data_ptr = unsafe { (*(ptr as *const PyroInner)).data_ptr() };

        if let Err(parse_error) = unsafe { PyroParser::check_raw(data_ptr) } {
            tracing::error!(?parse_error, "Checks failed for an FFI PyroBufPtr");
            let error = CapturedError::new(format!(
                "CRITICAL ERROR: Unable to construct a Ffi buffer due to {}",
                parse_error
            ))
            .with_location(std::panic::Location::caller())
            .with_backtrace(std::backtrace::Backtrace::force_capture());
            return Err(PyroError::HeaderFfi(error.into()));
        }
        let ptr = unsafe { NonNull::new_unchecked(ptr) };

        Ok(Self {
            view: ptr,
            dropper: pointer.dropper,
        })
    }

    pub fn clone_from_pyro<T: PyroData>(source: &T) -> Self {
        let mut vec = PyroVec::with_capacity(source.len());
        vec.extend_from_slice(&*source);
        vec.header_mut().copy_from_slice(source.header());
        Self::from(vec)
    }
}

impl From<PyroVec> for PyroBuf {
    fn from(vec: PyroVec) -> Self {
        let buf = PyroBuf {
            view: vec.view,
            dropper: vec.dropper,
        };
        std::mem::forget(vec);
        buf
    }
}

impl From<PyroVecPtr> for PyroBufPtr {
    fn from(vec_ptr: PyroVecPtr) -> Self {
        PyroBufPtr {
            ptr: vec_ptr.ptr,
            dropper: vec_ptr.dropper,
        }
    }
}

impl Deref for PyroBuf {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        &self.as_packet_slice()[PyroParser::HEADER_SIZE..]
    }
}

impl DerefMut for PyroBuf {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.as_packet_slice_mut()[PyroParser::HEADER_SIZE..]
    }
}

impl Drop for PyroBuf {
    fn drop(&mut self) {
        unsafe {
            let inner = self.view.as_ref();
            let base_ptr = self.view.as_ptr() as *mut u8;
            let cap = inner.capacity;

            // Single deallocation pass via the FFI safe dropper
            (self.dropper)(base_ptr, cap);
        }
    }
}

impl fmt::Debug for PyroBuf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PyroBuf")
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

// ============================================================================
// PyroVec implementation
// ============================================================================

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
            let inner_ptr = raw as *mut PyroInner;
            ptr::write(inner_ptr, PyroInner::new(capacity as u32));

            // Write UNIT_BYTES to the start of the data segment
            ptr::copy_nonoverlapping(
                UNIT_BYTES.as_ptr(),
                (*inner_ptr).data_ptr(),
                UNIT_BYTES.len(),
            );
            inner_ptr
        };

        Self {
            view: unsafe { NonNull::new_unchecked(raw_ptr) },
            dropper: Self::default_dropper,
            grower: Self::default_grower,
        }
    }

    pub fn into_raw(self) -> PyroVecPtr {
        let ptr = self.view.as_ptr();
        let dropper = self.dropper;
        let grower = self.grower;
        std::mem::forget(self);
        PyroVecPtr {
            ptr,
            dropper,
            grower,
        }
    }

    pub unsafe fn from_raw(pointer: PyroVecPtr) -> PyroResult<Self> {
        let view = pointer.ptr;
        if view.is_null() {
            return Err(PyroError::null_pointer());
        }

        let data_ptr = unsafe { (*view).data_ptr() };
        if let Err(parse_error) = unsafe { PyroParser::check_raw(data_ptr) } {
            tracing::error!(?parse_error, "Checks failed for an FFI PyroVecPtr");
            let error = CapturedError::new(format!(
                "CRITICAL ERROR: Unable to construct a Ffi vector due to {}",
                parse_error
            ))
            .with_location(std::panic::Location::caller())
            .with_backtrace(std::backtrace::Backtrace::force_capture());
            return Err(PyroError::HeaderFfi(error.into()));
        }
        Ok(Self {
            view: unsafe { NonNull::new_unchecked(view) },
            dropper: pointer.dropper,
            grower: pointer.grower,
        })
    }

    pub fn into_buf(self) -> PyroBuf {
        PyroBuf::from(self)
    }

    pub fn data_ptr(&self) -> *const u8 {
        let inner = unsafe { self.view.as_ref() };
        unsafe { inner.data_ptr().add(PyroParser::HEADER_SIZE) }
    }

    pub fn raw_ptr(&self) -> *const u8 {
        let inner = unsafe { self.view.as_ref() };
        unsafe { inner.data_ptr() }
    }

    pub(crate) fn as_packet_slice(&self) -> &[u8] {
        unsafe {
            slice::from_raw_parts(
                self.raw_ptr(),
                PyroParser::HEADER_SIZE + self.header_len() as usize,
            )
        }
    }

    pub(crate) fn as_packet_slice_mut(&mut self) -> &mut [u8] {
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

    pub fn as_raw_slice(&self) -> &[u8] {
        unsafe {
            slice::from_raw_parts(self.raw_ptr(), self.header_len() as usize)
        }
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
        &self.as_packet_slice()[PyroParser::HEADER_SIZE..]
    }
}

impl DerefMut for PyroVec {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.as_packet_slice_mut()[PyroParser::HEADER_SIZE..]
    }
}

impl Drop for PyroVec {
    fn drop(&mut self) {
        unsafe {
            let inner = self.view.as_ref();
            let ref_count = inner.ref_count.load(Ordering::Acquire);

            if ref_count == 0 {
                let base_ptr = self.view.as_ptr() as *mut u8;
                let cap = inner.capacity;
                (self.dropper)(base_ptr, cap);
            } else {
                if cfg!(debug_assertions) {
                    panic!(
                        "CRITICAL ERROR: Dropping PyroVec while {} references (PyroView) still exist. Memory leaked.",
                        ref_count
                    );
                } else {
                    tracing::error!(
                        ref_count = ref_count,
                        "CRITICAL ERROR: Dropping PyroVec while references still exist. Memory leaked."
                    );
                }
            }
        }
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
// PyroViewPtr — the completely FFI-safe, thin representation
// ============================================================================

#[repr(C)]
pub struct PyroViewPtr {
    pub ptr: *mut PyroInner,
}

// ============================================================================
// PyroView
// ============================================================================

/// A temporary, zero-copy view into a PyroVec residing in a byte slice
/// (e.g., WASM memory or a memory-mapped file).
///
/// Note: This view holds a raw pointer and does not track the lifetime
/// of the underlying memory. The caller must ensure the memory remains valid.
#[derive(Clone, Copy)]
pub struct PyroView {
    pub(crate) inner: NonNull<PyroInner>,
}

unsafe impl Send for PyroView {}
unsafe impl Sync for PyroView {}

impl PyroData for PyroView {
    #[inline]
    fn header(&self) -> &[u8; 16] {
        unsafe {
            let inner = self.inner.as_ref();
            &*(inner.data_ptr() as *const [u8; 16])
        }
    }

    fn capacity(&self) -> usize {
        unsafe {
            let inner = self.inner.as_ref();
            (inner.capacity as usize) - PyroParser::HEADER_SIZE
        }
    }
}

impl TryFrom<&[u8]> for PyroView {
    type Error = ParseError;

    fn try_from(slice: &[u8]) -> Result<Self, Self::Error> {
        // Assume the slice starts exactly at the PyroInner.
        get_view(slice, 0).map_err(|e| match e {
            PyroError::Header(p) => p,
            _ => ParseError::MisalignedPointer,
        })
    }
}

impl PyroView {
    /// Reconstructs a `PyroView` from a raw `PyroViewPtr`.
    ///
    /// # Safety
    /// * `raw.ptr` must be non-null and properly aligned (16-byte alignment).
    /// * The memory referenced must remain valid for the duration this view is used.
    pub unsafe fn from_ptr(raw: PyroViewPtr) -> Result<Self, PyroError> {
        let ptr = raw.ptr;
        if ptr.is_null() {
            return Err(PyroError::null_pointer());
        }

        let data_ptr = unsafe { (*ptr).data_ptr() };

        if let Err(parse_error) = unsafe { PyroParser::check_raw(data_ptr) } {
            tracing::error!(?parse_error, "Checks failed for an FFI PyroViewPtr");
            let error = CapturedError::new(format!(
                "CRITICAL ERROR: Unable to construct a Ffi view due to {}",
                parse_error
            ))
            .with_location(std::panic::Location::caller())
            .with_backtrace(std::backtrace::Backtrace::force_capture());
            return Err(PyroError::HeaderFfi(error.into()));
        };

        Ok(Self {
            inner: unsafe { NonNull::new_unchecked(ptr) },
        })
    }

    pub fn as_slice(&self) -> &[u8] {
        unsafe {
            let data = self.inner.as_ref().data_ptr().add(PyroParser::HEADER_SIZE);
            slice::from_raw_parts(data, self.header_len() as usize)
        }
    }

    pub fn as_raw_slice(&self) -> &[u8] {
        unsafe {
            let data = self.inner.as_ref().data_ptr();
            slice::from_raw_parts(data, self.header_len() as usize)
        }
    }

    pub fn ptr(&self) -> PyroViewPtr {
        PyroViewPtr {
            ptr: self.inner.as_ptr(),
        }
    }

    pub fn clone_to_vec(&self) -> PyroVec {
        let mut vec = PyroVec::clone_from_pyro(self);
        vec.extend_from_slice(self.as_slice());
        vec
    }
}

impl From<&PyroVec> for PyroView {
    fn from(value: &PyroVec) -> Self {
        value.view()
    }
}

impl PyroVec {
    pub fn view(&self) -> PyroView {
        PyroView { inner: self.view }
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

/// Creates a `PyroView` from a raw memory slice and an offset.
///
/// This performs bounds checks and header validation assuming the offset
/// points to a 16-byte `PyroInner` struct, followed immediately by the Data Header.
///
/// # Arguments
///
/// * `wasm_memory` - The entire available memory buffer.
/// * `offset` - The index into `wasm_memory` where the `PyroInner` struct begins.
pub fn get_view(wasm_memory: &[u8], offset: usize) -> Result<PyroView, PyroError> {
    // 1. We need at least 16 bytes for PyroInner + 16 bytes for the Data Header
    let inner_size = 16;
    if wasm_memory.len() < offset + inner_size + PyroParser::HEADER_SIZE {
        return Err(ParseError::SliceTooSmall.into());
    }
    if offset % 16 != 0 {
        return Err(ParseError::MisalignedPointer.into());
    }

    // 2. Map the pointer into the Wasm memory space safely
    let raw_ptr = unsafe { wasm_memory.as_ptr().add(offset) as *mut PyroInner };

    // 3. Read the payload length from the Pyro header
    let data_ptr = unsafe { (*raw_ptr).data_ptr() };
    let payload_len = unsafe {
        data_ptr.add(PyroParser::OFFSET_LEN).cast::<u32>().read_unaligned() as usize
    };

    // 4. Validate total bounds
    let total_required = inner_size + PyroParser::HEADER_SIZE + payload_len;
    if wasm_memory.len() - offset < total_required {
        return Err(ParseError::LengthExceedsCapacity.into());
    }

    // 5. Verify the actual Pyro Data Header
    let data_header_ptr = unsafe { (*raw_ptr).data_ptr() };
    let validation_slice =
        unsafe { slice::from_raw_parts(data_header_ptr, PyroParser::HEADER_SIZE + payload_len) };
    PyroParser::check(validation_slice)?;

    // 6. Construct View
    Ok(PyroView {
        inner: unsafe { NonNull::new_unchecked(raw_ptr) },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::header::DataStatus;

    #[test]
    fn test_view_valid_buffer() {
        let mut bv = PyroVec::with_capacity(32);
        bv.extend_from_slice(b"hello world");

        // We need to simulate Wasm memory encompassing the PyroInner block
        let full_len = 16 + PyroParser::HEADER_SIZE + bv.len();
        let memory = unsafe {
            let inner_ptr = bv.view.as_ptr() as *const u8;
            std::slice::from_raw_parts(inner_ptr, full_len)
        };

        let view = get_view(memory, 0).expect("Should create view");

        assert_eq!(view.len(), 11);
        assert_eq!(&*view, b"hello world");
        assert_eq!(view.status(), Ok(DataStatus::Empty));
    }

    #[test]
    fn test_view_bounds_check() {
        // Too small for PyroInner + Header (needs at least 32 bytes)
        let memory = vec![0u8; 20];
        let err = get_view(&memory, 0).unwrap_err();
        match err {
            PyroError::Header(ParseError::SliceTooSmall) => {}
            _ => panic!("Expected SliceTooSmall"),
        }
    }
}
