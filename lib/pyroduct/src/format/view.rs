use crate::{
    CapturedError, PyroError,
    format::{
        ParseError, PyroVec,
        header::{self, MutPyroData, PyroData, PyroHeader, PyroParser},
    },
};
use std::{
    fmt,
    ops::{Deref, DerefMut},
    slice,
};

/// A temporary, zero-copy view into a PyroVec residing in a byte slice
/// (e.g., WASM memory or a memory-mapped file).
///
/// This struct holds a reference to the exact slice of memory containing
/// the Header and the Data payload.
#[derive(Clone, Copy)]
pub struct PyroView<'a> {
    // The slice containing [Header (16 bytes) | Payload (len bytes)]
    pub(crate) raw_slice: &'a [u8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PyroViewPtr {
    pub(crate) ptr: *const u8,
    pub(crate) len: usize,
}

unsafe impl Send for PyroViewPtr {}
unsafe impl Sync for PyroViewPtr {}

impl<'a> PyroData for PyroView<'a> {
    #[inline]
    fn header(&self) -> &[u8; 16] {
        // Safety: The constructor `get_view` guarantees the slice is at least
        // 16 bytes (HEADER_SIZE). We can safely cast the pointer to a reference to an array.
        unsafe { &*(self.raw_slice.as_ptr() as *const [u8; 16]) }
    }

    fn capacity(&self) -> usize {
        self.raw_slice.len() - 16
    }
}

impl<'a> TryFrom<&'a [u8]> for PyroView<'a> {
    type Error = ParseError;

    fn try_from(slice: &'a [u8]) -> Result<Self, Self::Error> {
        header::PyroParser::check(slice)?;
        Ok(Self { raw_slice: slice })
    }
}

impl PyroView<'_> {
    /// Reconstructs a `PyroView` from a raw `PyroViewPtr`.
    ///
    /// # Safety
    /// * `raw.ptr` must be non-null and properly aligned (16-byte alignment).
    /// * `raw.ptr` must point to initialized memory of at least `raw.len` bytes.
    /// * The memory referenced must remain valid for the lifetime `'a` of the returned view.
    pub unsafe fn from_ptr(raw: PyroViewPtr) -> Result<Self, PyroError> {
        if let Err(parse_error) = unsafe { PyroParser::check_raw(raw.ptr) } {
            tracing::error!(?parse_error, "Checks failed for an FFI PyroBufPtr");
            let error = CapturedError::new(format!(
                "CRITICAL ERROR: Unable to construct a Ffi buffer due to {}",
                parse_error
            ))
            .with_location(std::panic::Location::caller())
            .with_backtrace(std::backtrace::Backtrace::force_capture());
            return Err(PyroError::HeaderFfi(error.into()));
        };
        let raw_slice = unsafe { slice::from_raw_parts(raw.ptr, raw.len) };

        let view = Self { raw_slice };
        if raw_slice.len() != view.header_len() as usize + 16 {
            let error = CapturedError::new(
                "CRITICAL ERROR: Unable to construct a Ffi view due to incorrect length",
            )
            .with_location(std::panic::Location::caller())
            .with_backtrace(std::backtrace::Backtrace::force_capture());
            return Err(PyroError::HeaderFfi(error.into()));
        }

        Ok(view)
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.raw_slice[PyroParser::HEADER_SIZE..]
    }

    pub fn ptr(&self) -> PyroViewPtr {
        let raw = PyroViewPtr {
            ptr: self.raw_slice.as_ptr(),
            len: self.raw_slice.len(),
        };
        raw
    }
}

impl<'a> From<&'a PyroVec> for PyroView<'a> {
    fn from(value: &'a PyroVec) -> Self {
        Self {
            raw_slice: value.as_packet_slice(),
        }
    }
}

impl PyroVec {
    pub fn view(&self) -> PyroView<'_> {
        PyroView {
            raw_slice: self.as_packet_slice(),
        }
    }
}

impl Deref for PyroView<'_> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.raw_slice[PyroParser::HEADER_SIZE..]
    }
}

impl fmt::Debug for PyroView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PyroView")
            .field("len", &self.len())
            .field("capacity", &self.capacity())
            .field("status", &self.status())
            .field("wire_fmt", &self.wire_format())
            .field("usr_ver", &self.version())
            .field("err_ver", &self.error_version())
            .finish()
    }
}

impl<'a> From<PyroView<'a>> for PyroViewPtr {
    fn from(view: PyroView<'a>) -> Self {
        Self {
            ptr: view.raw_slice.as_ptr(),
            len: view.raw_slice.len(),
        }
    }
}

pub struct PyroMutView<'a> {
    // The slice containing [Header (16 bytes) | Payload (len bytes)]
    raw_slice: &'a mut [u8],
}

impl<'a> PyroMutView<'a> {
    // Todo: This should check the capacity and all that
    pub fn copy_from_view(&mut self, view: &PyroView<'_>) -> Result<(), PyroError> {
        self.header_mut().copy_from_slice(view.header());
        self.copy_from_slice(&*view);
        Ok(())
    }
}

impl<'a> From<&'a mut PyroVec> for PyroView<'a> {
    fn from(value: &'a mut PyroVec) -> Self {
        Self {
            raw_slice: value.as_packet_slice_mut(),
        }
    }
}

impl<'a> PyroData for PyroMutView<'a> {
    fn capacity(&self) -> usize {
        self.raw_slice.len() - 16
    }
    #[inline]
    fn header(&self) -> &[u8; 16] {
        // Safety: The constructor guarantees the slice is at least 16 bytes.
        unsafe { &*(self.raw_slice.as_ptr() as *const [u8; 16]) }
    }
}

impl MutPyroData for PyroMutView<'_> {
    #[inline]
    fn header_mut(&mut self) -> &mut [u8; 16] {
        // Safety: The constructor guarantees the slice is at least 16 bytes.
        unsafe { &mut *(self.raw_slice.as_mut_ptr() as *mut [u8; 16]) }
    }
}

impl PyroVec {
    pub fn mut_view(&mut self) -> PyroMutView<'_> {
        PyroMutView {
            raw_slice: self.as_packet_slice_mut(),
        }
    }
}

impl Deref for PyroMutView<'_> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.raw_slice[PyroParser::HEADER_SIZE..]
    }
}

impl DerefMut for PyroMutView<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.raw_slice[PyroParser::HEADER_SIZE..]
    }
}

impl fmt::Debug for PyroMutView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PyroMutView")
            .field("len", &self.len())
            .field("capacity", &self.capacity())
            .field("status", &self.status())
            .field("wire_fmt", &self.wire_format())
            .field("usr_ver", &self.version())
            .field("err_ver", &self.error_version())
            .finish()
    }
}

/// Creates a `PyroView` from a raw memory slice and an offset.
///
/// This performs bounds checks and header validation.
///
/// # Arguments
///
/// * `wasm_memory` - The entire available memory buffer.
/// * `ptr_offset` - The index into `wasm_memory` where the PyroVec starts.
pub fn get_view<'a>(wasm_memory: &'a [u8], offset: usize) -> Result<PyroView<'a>, PyroError> {
    if wasm_memory.len() < 16 {
        return Err(ParseError::SliceTooSmall.into());
    }
    if offset % 16 != 0 {
        return Err(ParseError::MisalignedPointer.into());
    }
    let len_bytes: [u8; 4] = wasm_memory
        [offset + PyroParser::OFFSET_LEN..offset + PyroParser::OFFSET_LEN + 4]
        .try_into()
        .unwrap();
    let len = u32::from_le_bytes(len_bytes) as usize;

    // 5. Check Total Payload Bounds
    let total_len = PyroParser::HEADER_SIZE + len;
    let required_end = offset
        .checked_add(total_len)
        .ok_or(ParseError::LengthExceedsCapacity)?;
    if required_end > wasm_memory.len() {
        return Err(ParseError::LengthExceedsCapacity.into());
    }
    let raw_slice = &wasm_memory[offset..required_end];
    PyroParser::check(raw_slice)?;

    // 6. Construct View
    Ok(PyroView { raw_slice })
}

/// Creates a `PyroView` from a raw memory slice and an offset.
///
/// This performs bounds checks and header validation.
///
/// # Arguments
///
/// * `wasm_memory` - The entire available memory buffer.
/// * `ptr_offset` - The index into `wasm_memory` where the PyroVec starts.
pub fn get_view_mut<'a>(
    wasm_memory: &'a mut [u8],
    offset: usize,
) -> Result<PyroMutView<'a>, PyroError> {
    if wasm_memory.len() < 16 {
        return Err(ParseError::SliceTooSmall.into());
    }
    if offset % 16 != 0 {
        return Err(ParseError::MisalignedPointer.into());
    }
    let len_bytes: [u8; 4] = wasm_memory
        [offset + PyroParser::OFFSET_LEN..offset + PyroParser::OFFSET_LEN + 4]
        .try_into()
        .unwrap();
    let len = u32::from_le_bytes(len_bytes) as usize;

    // 5. Check Total Payload Bounds
    let total_len = PyroParser::HEADER_SIZE + len;
    let required_end = offset
        .checked_add(total_len)
        .ok_or(ParseError::LengthExceedsCapacity)?;
    if required_end > wasm_memory.len() {
        return Err(ParseError::LengthExceedsCapacity.into());
    }
    let raw_slice = &mut wasm_memory[offset..required_end];
    PyroParser::check(raw_slice)?;

    // 6. Construct View
    Ok(PyroMutView { raw_slice })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::header::DataStatus;

    #[test]
    fn test_view_valid_buffer() {
        let mut bv = PyroVec::with_capacity(32);
        bv.extend_from_slice(b"hello world");

        let memory = bv.as_packet_slice();

        let view = get_view(memory, 0).expect("Should create view");

        assert_eq!(view.len(), 11);
        assert_eq!(&*view, b"hello world");
        assert_eq!(view.status(), Ok(DataStatus::Empty));
    }

    #[test]
    fn test_view_bounds_check() {
        let memory = vec![0u8; 10]; // Too small for header
        let err = get_view(&memory, 0).unwrap_err();
        match err {
            PyroError::Header(ParseError::SliceTooSmall) => {}
            _ => panic!("Expected OutOfBounds"),
        }
    }

    #[test]
    fn test_view_magic_check() {
        let mut memory = vec![0u8; 32];
        // Offset 0 is aligned
        // Write garbage magic
        memory[0] = 0xFF;

        let err = get_view(&memory, 0).unwrap_err();
        match err {
            PyroError::Header(ParseError::InvalidMagic) => {}
            _ => panic!("Expected InvalidHeader"),
        }
    }
}
