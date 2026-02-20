use std::alloc::{self, Layout};
use std::hash::Hasher;
use std::ops::{Deref, DerefMut};
use std::ptr::{self, NonNull};
use std::{fmt, io, slice};

use crate::header::{
    OwnedPyroData, PyroData, PyroHeader, PyroHeaderMut, PyroParser, UNIT_BYTES, UNIT_HEADER,
};
use crate::{CapturedError, PyroError, PyroResult};

// ============================================================================
// Function pointer types for FFI-safe memory management
// ============================================================================

type PyroVecDropper = unsafe extern "C" fn(ptr: *mut u8, capacity: usize);
type PyroVecGrower =
    unsafe extern "C" fn(ptr: *mut u8, old_capacity: usize, new_capacity: usize) -> *mut u8;

// ============================================================================
// PyroVec
// ============================================================================

/// A 16-byte aligned buffer with a self-describing header.
/// Compatible with FFI passing as a raw pointer or TCP/Unix framing.
pub struct PyroVec {
    ptr: NonNull<u8>,
    capacity: usize,
    dropper: PyroVecDropper,
    grower: PyroVecGrower,
}

// ============================================================================
// PyroVecPtr — the FFI-safe representation
// ============================================================================

#[repr(C)]
pub struct PyroVecPtr {
    ptr: *mut u8,
    capacity: usize,
    dropper: PyroVecDropper,
    grower: PyroVecGrower,
}

unsafe impl std::marker::Send for PyroVecPtr {}

// ============================================================================
// PyroBuf — a PyroVec with the grow capability removed
// ============================================================================

/// A mutable buffer that owns its allocation but cannot grow.
///
/// Created by consuming a `PyroVec` via `PyroVec::into_buf()` or
/// `PyroBuf::from(pyro_vec)`. The header and existing data remain
/// fully mutable, but `push` / `extend_from_slice` are not available.
pub struct PyroBuf {
    ptr: NonNull<u8>,
    capacity: usize,
    dropper: PyroVecDropper,
}

/// FFI-safe representation of a `PyroBuf`.
///
/// Like `PyroVecPtr` but without the grower — suitable for passing
/// fixed-capacity buffers across FFI boundaries.
#[repr(C)]
pub struct PyroBufPtr {
    ptr: *mut u8,
    capacity: usize,
    dropper: PyroVecDropper,
}

// SAFETY: PyroBuf owns its allocation exclusively
unsafe impl Send for PyroBuf {}
unsafe impl Sync for PyroBuf {}

impl PyroData for PyroBuf {
    #[inline]
    fn header(&self) -> &[u8; 16] {
        unsafe { &*(self.ptr.as_ptr() as *const [u8; 16]) }
    }
    fn capacity(&self) -> usize {
        self.capacity - PyroParser::HEADER_SIZE
    }
}

impl OwnedPyroData for PyroBuf {
    #[inline]
    fn header_mut(&mut self) -> &mut [u8; 16] {
        unsafe { &mut *(self.ptr.as_ptr() as *mut [u8; 16]) }
    }
}

impl PyroBuf {
    /// Returns the raw pointer to the start of the data section (after header).
    pub fn data_ptr(&self) -> *const u8 {
        unsafe { self.ptr.as_ptr().add(PyroParser::HEADER_SIZE) }
    }

    /// Returns a slice over the data payload.
    pub fn as_slice(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.data_ptr(), self.len()) }
    }

    /// Returns a mutable slice over the data payload.
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe {
            slice::from_raw_parts_mut(self.ptr.as_ptr().add(PyroParser::HEADER_SIZE), self.len())
        }
    }

    /// Returns a slice containing the Header (16 bytes) AND the Data (len bytes).
    pub(crate) fn as_packet_slice(&self) -> &[u8] {
        unsafe {
            slice::from_raw_parts(
                self.ptr.as_ptr(),
                PyroParser::HEADER_SIZE + self.header_len() as usize,
            )
        }
    }

    /// Returns a mutable slice containing the Header (16 bytes) AND the Data (len bytes).
    pub(crate) fn as_packet_slice_mut(&self) -> &mut [u8] {
        unsafe {
            slice::from_raw_parts_mut(
                self.ptr.as_ptr(),
                PyroParser::HEADER_SIZE + self.header_len() as usize,
            )
        }
    }

    /// Resets the data length to zero without deallocating.
    pub fn clear(&mut self) {
        unsafe { self.set_len(0) };
    }

    /// Consumes self and returns the raw FFI-safe pointer.
    ///
    /// Caller is responsible for eventually reconstructing via `from_raw`
    /// and dropping, or manually deallocating with the correct layout.
    pub fn into_raw(self) -> PyroBufPtr {
        let raw = PyroBufPtr {
            ptr: self.ptr.as_ptr(),
            capacity: self.capacity,
            dropper: self.dropper,
        };
        std::mem::forget(self);
        raw
    }

    /// Reconstructs a `PyroBuf` from a raw `PyroBufPtr`.
    ///
    /// # Safety
    /// Caller ensures pointer is valid, non-null, and owned by the caller.
    #[track_caller]
    pub unsafe fn from_raw(pointer: PyroBufPtr) -> PyroResult<Self> {
        let ptr = pointer.ptr;
        if let Err(parse_error) = unsafe { PyroParser::check_raw(ptr) } {
            tracing::error!(?parse_error, "Checks failed for an FFI PyroBufPtr");
            let error = CapturedError::new(format!(
                "CRITICAL ERROR: Unable to construct a Ffi buffer due to {}",
                parse_error
            ))
            .with_location(std::panic::Location::caller())
            .with_backtrace(std::backtrace::Backtrace::force_capture());
            return Err(PyroError::HeaderFfi(error.into()));
        }
        let ptr = unsafe { NonNull::new_unchecked(ptr as *mut u8) };
        Ok(Self {
            ptr,
            capacity: pointer.capacity,
            dropper: pointer.dropper,
        })
    }

    /// Creates a new PyroBuf by cloning the header and data from any
    /// type that implements PyroData.
    pub fn clone_from_pyro<T: PyroData>(source: &T) -> Self {
        // Use PyroVec's allocation logic to ensure proper 16-byte alignment
        let mut vec = PyroVec::with_capacity(source.len());

        // Copy the data payload
        vec.extend_from_slice(&*source);

        // Copy the header metadata (status, versions, etc.)
        vec.header_mut().copy_from_slice(source.header());

        // Convert to PyroBuf (this consumes the PyroVec and removes growth capability)
        Self::from(vec)
    }
}

impl From<PyroVec> for PyroBuf {
    fn from(vec: PyroVec) -> Self {
        let buf = PyroBuf {
            ptr: vec.ptr,
            capacity: vec.capacity,
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
            capacity: vec_ptr.capacity,
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
            (self.dropper)(self.ptr.as_ptr(), self.capacity);
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
            .field("usr_ver", &self.version())
            .field("err_ver", &self.error_version())
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
        unsafe { &*(self.ptr.as_ptr() as *const [u8; 16]) }
    }
    fn capacity(&self) -> usize {
        self.capacity - PyroParser::HEADER_SIZE
    }
}

// SAFETY: PyroVec owns its allocation exclusively
// and contains no references to thread-local state
unsafe impl Send for PyroVec {}
unsafe impl Sync for PyroVec {}

impl OwnedPyroData for PyroVec {
    #[inline]
    fn header_mut(&mut self) -> &mut [u8; 16] {
        unsafe { &mut *(self.ptr.as_ptr() as *mut [u8; 16]) }
    }
}

// ============================================================================
// PyroVec implementation
// ============================================================================

impl PyroVec {
    extern "C" fn default_dropper(ptr: *mut u8, capacity: usize) {
        unsafe {
            let layout = Layout::from_size_align(capacity, PyroParser::ALIGN)
                .expect("Layout logic mismatch in dropper");
            alloc::dealloc(ptr, layout);
        }
    }

    unsafe extern "C" fn default_grower(
        ptr: *mut u8,
        old_capacity: usize,
        new_capacity: usize,
    ) -> *mut u8 {
        unsafe {
            let old_layout = Layout::from_size_align(old_capacity, PyroParser::ALIGN).unwrap();
            let new_ptr = alloc::realloc(ptr, old_layout, new_capacity);
            if new_ptr.is_null() {
                alloc::handle_alloc_error(
                    Layout::from_size_align(new_capacity, PyroParser::ALIGN).unwrap(),
                );
            }
            new_ptr
        }
    }

    /// Creates a new vector with a specific capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        let capacity = capacity + PyroParser::HEADER_SIZE;
        let layout =
            Layout::from_size_align(capacity, PyroParser::ALIGN).expect("Invalid layout alignment");

        let ptr = unsafe {
            let raw = alloc::alloc(layout);
            if raw.is_null() {
                alloc::handle_alloc_error(layout);
            }
            ptr::copy_nonoverlapping(UNIT_BYTES.as_ptr(), raw as *mut u8, UNIT_BYTES.len());
            NonNull::new_unchecked(raw)
        };
        Self {
            ptr,
            capacity,
            dropper: Self::default_dropper,
            grower: Self::default_grower,
        }
    }

    /// Consumes self and returns the raw FFI-safe pointer.
    ///
    /// Caller is responsible for eventually reconstructing via `from_raw`
    /// and dropping, or manually deallocating with the correct layout.
    pub fn into_raw(self) -> PyroVecPtr {
        let ptr = self.ptr.as_ptr();
        let capacity = self.capacity;
        let dropper = self.dropper;
        let grower = self.grower;
        std::mem::forget(self);
        PyroVecPtr {
            ptr,
            capacity,
            dropper,
            grower,
        }
    }

    /// Reconstructs a `PyroVec` from a raw `PyroVecPtr`.
    ///
    /// # Safety
    /// Caller ensures pointer is valid, non-null, and owned by the caller.
    #[track_caller]
    pub unsafe fn from_raw(pointer: PyroVecPtr) -> PyroResult<Self> {
        let ptr = pointer.ptr;
        if let Err(parse_error) = unsafe { PyroParser::check_raw(ptr) } {
            tracing::error!(?parse_error, "Checks failed for an FFI PyroVecPtr");
            let error = CapturedError::new(format!(
                "CRITICAL ERROR: Unable to construct a Ffi vector due to {}",
                parse_error
            ))
            .with_location(std::panic::Location::caller())
            .with_backtrace(std::backtrace::Backtrace::force_capture());
            return Err(PyroError::HeaderFfi(error.into()));
        }
        let ptr = unsafe { NonNull::new_unchecked(ptr as *mut u8) };
        let capacity = pointer.capacity;
        let dropper = pointer.dropper;
        let grower = pointer.grower;
        Ok(Self {
            ptr,
            capacity,
            dropper,
            grower,
        })
    }

    /// Consumes self and returns a `PyroBuf` (drops the grow capability).
    pub fn into_buf(self) -> PyroBuf {
        PyroBuf::from(self)
    }

    pub fn data_ptr(&self) -> *const u8 {
        unsafe { self.ptr.as_ptr().add(PyroParser::HEADER_SIZE) }
    }

    // --- Vec Operations ---

    /// Returns a slice containing the Header (16 bytes) AND the Data (len bytes).
    pub(crate) fn as_packet_slice(&self) -> &[u8] {
        unsafe {
            slice::from_raw_parts(
                self.ptr.as_ptr(),
                PyroParser::HEADER_SIZE + self.header_len() as usize,
            )
        }
    }

    /// Returns a mutable slice containing the Header (16 bytes) AND the Data (len bytes).
    pub(crate) fn as_packet_slice_mut(&mut self) -> &mut [u8] {
        unsafe {
            slice::from_raw_parts_mut(
                self.ptr.as_ptr(),
                PyroParser::HEADER_SIZE + self.header_len() as usize,
            )
        }
    }

    pub fn push(&mut self, byte: u8) {
        if self.header_len() as usize == self.capacity() {
            self.grow(1);
        }
        unsafe {
            let len = self.len();
            let data_start = self.ptr.as_ptr().add(PyroParser::HEADER_SIZE);
            ptr::write(data_start.add(len), byte);
            self.set_len(len as u32 + 1);
        }
    }

    pub fn extend_from_slice(&mut self, other: &[u8]) {
        let required = other.len();
        let current_len = self.len();
        let current_cap = self.capacity();

        if current_len + required + PyroParser::HEADER_SIZE > current_cap {
            self.grow(required);
        }
        unsafe {
            ptr::copy_nonoverlapping(
                other.as_ptr(),
                self.ptr.as_ptr().add(PyroParser::HEADER_SIZE + current_len),
                required,
            );
            self.set_len((current_len + required) as u32);
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.data_ptr(), self.len()) }
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe {
            slice::from_raw_parts_mut(self.ptr.as_ptr().add(PyroParser::HEADER_SIZE), self.len())
        }
    }

    pub fn clear(&mut self) {
        unsafe { self.set_len(0) };
    }

    // --- Internals ---

    pub(crate) fn grow(&mut self, additional: usize) {
        let current_cap = self.capacity;
        let current_len = self.len();

        let required_cap = current_len
            .checked_add(additional)
            .and_then(|c| c.checked_add(PyroParser::HEADER_SIZE))
            .expect("capacity overflow");

        let mut new_cap = current_cap.saturating_mul(2).max(required_cap);

        let remainder = new_cap % PyroParser::ALIGN;
        if remainder != 0 {
            new_cap = new_cap
                .checked_add(PyroParser::ALIGN - remainder)
                .expect("capacity overflow during alignment");
        }

        unsafe {
            let new_ptr = (self.grower)(self.ptr.as_ptr(), current_cap, new_cap);
            // NOTE: null check / handle_alloc_error is the grower's responsibility
            self.capacity = new_cap;
            self.ptr = NonNull::new_unchecked(new_ptr);
        }
    }

    /// Returns a new, owned PyroVec representing Ok(()).
    /// This performs an allocation so it can be safely dropped later.
    pub fn ok() -> Self {
        let vec = Self::with_capacity(0);
        unsafe {
            ptr::copy_nonoverlapping(
                UNIT_HEADER.0.as_ptr(),
                vec.ptr.as_ptr(),
                PyroParser::HEADER_SIZE,
            );
        }
        vec
    }

    /// Creates a new PyroVec by cloning the header and data from any
    /// type that implements PyroData.
    pub fn clone_from_pyro<T: PyroData>(source: &T) -> Self {
        // 1. Allocate a new vector with the required capacity
        let mut new_vec = Self::with_capacity(source.len());

        // 2. Copy the data payload
        // Note: PyroData provides .as_slice() via common trait logic
        // but here we can use the source's length and data access.
        new_vec.extend_from_slice(&*source);

        // 3. Copy the header metadata
        // We use the raw header to ensure status, versions, and wire format are preserved
        new_vec.header_mut().copy_from_slice(source.header());

        new_vec
    }
}

// --- PyroVec Trait Implementations ---

impl Clone for PyroVec {
    fn clone(&self) -> Self {
        let mut new_vec = Self::with_capacity(self.len());
        new_vec.extend_from_slice(self.as_slice());
        new_vec.set_status_u8(self.status_u8());
        new_vec.set_wire_format(self.wire_format());
        new_vec.set_version(self.version());
        new_vec.set_error_version(self.error_version());
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
            .field("usr_ver", &self.version())
            .field("err_ver", &self.error_version())
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
            (self.dropper)(self.ptr.as_ptr(), self.capacity);
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

#[cfg(test)]
mod unit_tests {
    use super::*;
    use crate::header::{DataStatus, PyroData};

    #[test]
    fn test_pyro_vec_ok() {
        let vec = PyroVec::ok();

        // Check Header correctness
        assert_eq!(vec.len(), 0);
        assert_eq!(vec.capacity(), 0);
        assert_eq!(vec.status(), Ok(DataStatus::Empty));
        assert_eq!(vec.wire_format(), 1);

        // Check Memory Layout
        assert_eq!(vec.as_ptr() as usize % 16, 0, "Owned Ok must be aligned");

        // Ensure it is actually owned (drop shouldn't panic/segfault)
        drop(vec);
    }

    #[test]
    fn test_pyrovecptr_preserves_dropper() {
        // Custom dropper that sets a flag (for testing purposes)
        // Note: In real code, you'd typically use the default dropper
        static mut CUSTOM_DROPPER_CALLED: bool = false;

        unsafe extern "C" fn test_dropper(ptr: *mut u8, capacity: usize) {
            unsafe { CUSTOM_DROPPER_CALLED = true };
            // Still need to actually free the memory
            use std::alloc::{Layout, dealloc};
            let total_cap = capacity; // HEADER_SIZE
            let layout = Layout::from_size_align(total_cap, 16).unwrap();
            unsafe { dealloc(ptr, layout) };
        }

        let vec = PyroVec::with_capacity(100);
        let original_ptr = vec.as_ptr();

        // Convert to PyroVecPtr and modify dropper
        let mut ptr_struct = vec.into_raw();
        ptr_struct.dropper = test_dropper;

        // Reconstruct
        let reconstructed = unsafe { PyroVec::from_raw(ptr_struct) }.unwrap();

        // Verify pointer is the same
        assert_eq!(reconstructed.as_ptr(), original_ptr);

        // Drop it - this should call our custom dropper
        drop(reconstructed);

        // Verify our custom dropper was called
        unsafe {
            assert!(
                CUSTOM_DROPPER_CALLED,
                "Custom dropper should have been called"
            );
        }
    }

    #[test]
    fn test_from_raw_validates_null_pointer() {
        // Create a valid PyroVec and get its pointer structure
        let vector = PyroVec::with_capacity(100);
        let mut pointer = vector.into_raw();
        let original_ptr = pointer.ptr;
        // Corrupt the pointer field to be null (requires private access)
        pointer.ptr = std::ptr::null_mut();

        // from_raw should now catch this via check_raw validation
        let result = unsafe { PyroVec::from_raw(pointer) };

        match result {
            Err(PyroError::HeaderFfi(captured)) => {
                assert!(captured.message.contains("pointer is null"));
            }
            _ => panic!("Expected PyroError::HeaderFfi, got {:?}", result),
        }

        let cleanup_ptr = PyroVecPtr {
            ptr: original_ptr,
            capacity: 116,
            dropper: PyroVec::default_dropper,
            grower: PyroVec::default_grower,
        };
        let _ = unsafe { PyroVec::from_raw(cleanup_ptr) };
    }

    #[test]
    fn test_from_raw_validates_misaligned_pointer() {
        // Create a valid PyroVec
        let vector = PyroVec::with_capacity(100);
        let mut pointer = vector.into_raw();

        // Corrupt the pointer to be misaligned (add 1 to make it not 16-byte aligned)
        // Store original for cleanup
        let original_ptr = pointer.ptr;
        pointer.ptr = unsafe { pointer.ptr.add(1) };

        // from_raw should catch the misalignment
        let result = unsafe { PyroVec::from_raw(pointer) };

        match result {
            Err(PyroError::HeaderFfi(captured)) => {
                assert!(captured.message.contains("pointer is misaligned"));
            }
            _ => panic!("Expected PyroError::HeaderFfi, got {:?}", result),
        }

        // Restore and clean up properly
        let cleanup_ptr = PyroVecPtr {
            ptr: original_ptr,
            capacity: 116,
            dropper: PyroVec::default_dropper,
            grower: PyroVec::default_grower,
        };
        let _ = unsafe { PyroVec::from_raw(cleanup_ptr) };
    }

    #[test]
    fn test_from_raw_validates_bad_magic() {
        use std::ptr;

        // Create a valid PyroVec
        let vector = PyroVec::with_capacity(100);
        let pointer = vector.into_raw();
        let original_ptr = pointer.ptr;

        // Corrupt the magic number at the pointer location
        unsafe {
            ptr::write(pointer.ptr as *mut u32, 0xDEADBEEF);
        }

        // from_raw should catch the bad magic
        let result = unsafe { PyroVec::from_raw(pointer) };

        match result {
            Err(PyroError::HeaderFfi(captured)) => {
                assert!(captured.message.contains("invalid magic header"));
            }
            _ => panic!("Expected PyroError::HeaderFfi, got {:?}", result),
        }

        let cleanup_ptr = PyroVecPtr {
            ptr: original_ptr,
            capacity: 116,
            dropper: PyroVec::default_dropper,
            grower: PyroVec::default_grower,
        };
        let _ = unsafe { PyroVec::from_raw(cleanup_ptr) };
    }

    #[test]
    fn test_from_raw_with_zero_capacity() {
        // Test that a PyroVecPtr with zero capacity works
        let vector = PyroVec::with_capacity(0);
        assert_eq!(vector.capacity(), 0);
        let pointer = vector.into_raw();

        assert_eq!(pointer.capacity, 16);

        let reconstructed =
            unsafe { PyroVec::from_raw(pointer) }.expect("Zero capacity should be valid");
        assert_eq!(reconstructed.capacity(), 0);
        assert_eq!(reconstructed.len(), 0);
    }

    #[test]
    fn test_pyrovecptr_fields_accessible() {
        // Test that PyroVecPtr fields can be accessed (private access in unit tests)
        let vector = PyroVec::with_capacity(100);
        assert_eq!(vector.capacity(), 100);
        assert_eq!(vector.capacity, 116);
        let pointer = vector.into_raw();

        // These fields are accessible within the crate's tests
        assert!(!pointer.ptr.is_null());
        assert_eq!(pointer.capacity, 116);
        // dropper is a function pointer, just verify it exists
        assert_ne!(pointer.dropper as usize, 0);

        // Clean up
        let _vec = unsafe { PyroVec::from_raw(pointer) }.expect("Should reconstruct");
    }

    #[test]
    fn test_grow_maintains_alignment() {
        let mut vec = PyroVec::with_capacity(1);

        for _ in 0..4 {
            // Each iteration should trigger growth
            let current_cap = vec.capacity();
            while vec.len() < current_cap {
                vec.push(0);
            }
            vec.push(0); // Trigger grow

            let addr = vec.as_ptr() as usize;
            assert_eq!(addr % 16, 0, "Must remain 16-byte aligned after grow");
        }
    }
    // Slow with miri
    // #[test]
    // fn test_large_allocation() {
    //     let mut vec = PyroVec::with_capacity(1_000_000);
    //     let data = vec![0xABu8; 1_000_000];

    //     vec.extend_from_slice(&data);

    //     assert_eq!(vec.len(), 1_000_000);
    //     assert!(vec.as_slice().iter().all(|&b| b == 0xAB));
    // }

    #[test]
    fn test_mixed_operations() {
        let mut vec = PyroVec::with_capacity(10);

        vec.push(1);
        vec.extend_from_slice(&[2, 3, 4]);
        vec.push(5);
        vec.set_status_u8(100);
        vec.extend_from_slice(&[6, 7]);
        vec.push(8);

        assert_eq!(vec.as_slice(), &[1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(vec.status_u8(), 100);
    }

    #[test]
    fn test_clear_and_refill_multiple_times() {
        let mut vec = PyroVec::with_capacity(10);

        for round in 0..5 {
            vec.clear();
            for i in 0..50 {
                vec.push((round * 50 + i) as u8);
            }
            assert_eq!(vec.len(), 50);
        }

        // Final state should be last round's data
        assert_eq!(vec.len(), 50);
    }
}
