use crate::{
    BridgeError, BridgeVec,
    header::{BridgeData, BridgeHeader, BridgeHeaderMut, BridgeParser, MAGIC_VAL}
};
use rkyv::rancor::Strategy;
use rkyv::validation::{Validator, archive::ArchiveValidator, shared::SharedValidator};
use rkyv::bytecheck::CheckBytes;
use rkyv::Archive;
use thiserror::Error;
use std::{fmt::{self, Debug}, ops::Deref};

/// Errors that can occur when creating or accessing a BridgeView.
#[derive(Debug, Error)]
pub enum BridgeErrorView {
    /// The offset provided was not aligned to 16 bytes.
    #[error("misaligned pointer")]
    MisalignedPointer,

    /// The memory slice was too small to contain the header or the data.
    #[error("out of bounds: required {required} bytes, available {available}")]
    OutOfBounds { required: usize, available: usize },

    /// The magic bytes did not match the expected value.
    #[error("invalid magic header")]
    InvalidHeader,

    /// Rkyv validation failed.
    #[error("validation error: {0}")]
    Validation(#[from] rkyv::rancor::Error),

    /// The buffer contained a BridgeError (status code != 0).
    /// This wraps the standard BridgeError which may allocate.
    #[error("bridge error: {0}")]
    BridgeError(#[from] BridgeError),
}

/// A temporary, zero-copy view into a BridgeVec residing in a byte slice
/// (e.g., WASM memory or a memory-mapped file).
///
/// This struct holds a reference to the exact slice of memory containing
/// the Header and the Data payload.
#[derive(Clone, Copy)]
pub struct BridgeView<'a> {
    // The slice containing [Header (16 bytes) | Payload (len bytes)]
    raw_slice: &'a [u8],
}

impl BridgeData for BridgeView<'_> {
    #[inline]
    fn header(&self) -> &[u8; 16] {
        // Safety: The constructor `get_view` guarantees the slice is at least 
        // 16 bytes (HEADER_SIZE). We can safely cast the pointer to a reference to an array.
        unsafe { &*(self.raw_slice.as_ptr() as *const [u8; 16]) }
    }
}

impl Deref for BridgeView<'_> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.raw_slice[BridgeParser::HEADER_SIZE..]
    }
}

impl fmt::Debug for BridgeView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BridgeView")
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

impl<'a> BridgeView<'a> {
    /// Returns the slice containing the Header and the Data.
    #[inline]
    pub fn inner(&self) -> &'a [u8] {
        self.raw_slice
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.raw_slice[BridgeParser::HEADER_SIZE..]
    }

    pub fn capacity(&self) -> usize {
        &self.raw_slice.len() - BridgeParser::HEADER_SIZE
    }

    /// Creates an owned `BridgeVec` from this view by copying the data.
    /// This is useful when the view indicates an error that requires complex parsing,
    /// or when you need to keep the data alive past the lifetime of the view.
    pub fn to_owned(&self) -> BridgeVec {
        let mut vec = BridgeVec::with_capacity(self.len());
        vec.extend_from_slice(&*self);
        
        // Copy header fields to maintain protocol state
        vec.set_status_u8(self.status_u8());
        vec.set_wire_format(self.wire_format());
        vec.set_version(self.version());
        vec.set_error_version(self.error_version());
        
        vec
    }

    /// Converts the view into a `BridgeError`.
    /// 
    /// This should be called when `status() != 0`. It allocates a new `BridgeVec`
    /// (copying the data) and then uses the standard library logic to parse
    /// the error code and payload (e.g., deserializing JSON captured errors).
    pub fn parse_view_as_error(&self) -> BridgeError {
        self.to_owned().parse_as_error()
    }

    /// Parses the view into a TypedBridgeView, verifying the rkyv archive.
    ///
    /// If the status indicates a failure (non-zero), this returns an error.
    pub fn parse<T>(self) -> Result<TypedBridgeView<'a, T>, BridgeErrorView>
    where
        T: Archive,
        T::Archived: for<'b> CheckBytes<
            Strategy<Validator<ArchiveValidator<'b>, SharedValidator>, rkyv::rancor::Error>,
        >,
    {
        self.unchecked_parse::<T>()
    }

    /// Parses the view without checking the status code.
    /// Useful if you know the type matches the status (e.g. parsing an Error type).
    pub fn unchecked_parse<T>(self) -> Result<TypedBridgeView<'a, T>, BridgeErrorView>
    where
        T: Archive,
        T::Archived: for<'b> CheckBytes<
            Strategy<Validator<ArchiveValidator<'b>, SharedValidator>, rkyv::rancor::Error>,
        >,
    {
        let slice = self.as_slice();
        let archived_ref = rkyv::access::<T::Archived, rkyv::rancor::Error>(slice)
            .map_err(BridgeErrorView::Validation)?;
        // 3. Extend lifetime to 'a.
        //    SAFETY:
        //    - `BridgeVec` data is allocated on the heap via `alloc`.
        //    - Moving `self` into `TypedBuf` only moves the pointer (struct), not the heap data.
        //    - The heap address remains stable.
        //    - `TypedBuf` owns `vec` and does not expose mutable access to it, preventing reallocation.
        //    - Therefore, the reference into `vec` is valid as long as `TypedBuf` exists.
        let archived_elided =
            unsafe { std::mem::transmute::<&T::Archived, &'a T::Archived>(archived_ref) };

        Ok(TypedBridgeView {
            view: self,
            archived: archived_elided,
        })
    }
}

/// A type-safe wrapper around a BridgeView containing an archived rkyv type.
///
/// Unlike `TypedBuf`, this does not own the data. It borrows from the
/// underlying slice `'a`.
pub struct TypedBridgeView<'a, T>
where
    T: Archive,
{
    view: BridgeView<'a>,
    archived: &'a T::Archived,
}

impl<'a, T> Deref for TypedBridgeView<'a, T>
where
    T: Archive,
{
    type Target = T::Archived;

    fn deref(&self) -> &Self::Target {
        self.archived
    }
}

impl<'a, T> TypedBridgeView<'a, T>
where
    T: Archive,
{
    /// Gets the underlying raw view.
    pub fn into_inner(self) -> BridgeView<'a> {
        self.view
    }

    /// Deserializes the data into the native Rust type.
    pub fn deserialize(&self) -> Result<T, BridgeError>
    where
        T::Archived: rkyv::Deserialize<T, Strategy<rkyv::de::Pool, rkyv::rancor::Error>>,
    {
        rkyv::deserialize::<T, rkyv::rancor::Error>(self.archived)
            .map_err(BridgeError::deserialization)
    }
}

/// Creates a `BridgeView` from a raw memory slice and an offset.
///
/// This performs bounds checks and header validation.
///
/// # Arguments
///
/// * `wasm_memory` - The entire available memory buffer.
/// * `ptr_offset` - The index into `wasm_memory` where the BridgeVec starts.
pub fn get_view<'a>(
    wasm_memory: &'a [u8], 
    ptr_offset: u32
) -> Result<BridgeView<'a>, BridgeErrorView> {
    let offset = ptr_offset as usize;

    // 1. Check Alignment
    // BridgeVec mandates 16-byte alignment.
    if offset % BridgeParser::ALIGN != 0 {
        return Err(BridgeErrorView::MisalignedPointer);
    }

    // 2. Check Header Bounds
    // We need at least HEADER_SIZE bytes to read the header
    if offset.checked_add(BridgeParser::HEADER_SIZE).ok_or(BridgeErrorView::OutOfBounds { required: BridgeParser::HEADER_SIZE, available: 0 })? > wasm_memory.len() {
        return Err(BridgeErrorView::OutOfBounds {
            required: offset + BridgeParser::HEADER_SIZE,
            available: wasm_memory.len(),
        });
    }

    // 3. Read Header & Verify Magic
    // We can safely slice [offset..offset+4] because of check #2
    let magic_bytes: [u8; 4] = wasm_memory[offset + BridgeParser::OFFSET_MAGIC .. offset + BridgeParser::OFFSET_MAGIC + 4].try_into().unwrap();
    let magic = u32::from_le_bytes(magic_bytes);

    if magic != MAGIC_VAL {
        return Err(BridgeErrorView::InvalidHeader);
    }

    // 4. Read Length
    let len_bytes: [u8; 4] = wasm_memory[offset + BridgeParser::OFFSET_LEN .. offset + BridgeParser::OFFSET_LEN + 4].try_into().unwrap();
    let len = u32::from_le_bytes(len_bytes) as usize;

    // 5. Check Total Payload Bounds
    let total_len = BridgeParser::HEADER_SIZE + len;
    let required_end = offset.checked_add(total_len).ok_or(BridgeErrorView::OutOfBounds { required: usize::MAX, available: wasm_memory.len() })?;

    if required_end > wasm_memory.len() {
        return Err(BridgeErrorView::OutOfBounds {
            required: required_end,
            available: wasm_memory.len(),
        });
    }

    // 6. Construct View
    Ok(BridgeView {
        raw_slice: &wasm_memory[offset..required_end],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BridgeVec, header::DataStatus};

    #[test]
    fn test_view_valid_buffer() {
        // Create a real BridgeVec
        let mut bv = BridgeVec::with_capacity(32);
        bv.extend_from_slice(b"hello world");
        
        // Mock "wasm memory"
        let memory = bv.as_packet_slice();
        
        // Since BridgeVec allocates on heap, we can't guarantee `memory` is 16-byte aligned 
        // relative to 0 if we just take a slice, unless the allocator did so.
        // However, `BridgeVec` internal pointer is aligned. 
        // If we treat index 0 of `memory` as the pointer:
        let view = get_view(memory, 0).expect("Should create view");
        
        assert_eq!(view.len(), 11);
        assert_eq!(&*view, b"hello world");
        assert_eq!(view.status(), Ok(DataStatus::ValidData));
    }

    #[test]
    fn test_view_bounds_check() {
        let memory = vec![0u8; 10]; // Too small for header
        let err = get_view(&memory, 0).unwrap_err();
        match err {
            BridgeErrorView::OutOfBounds { .. } => {},
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
            BridgeErrorView::InvalidHeader => {},
            _ => panic!("Expected InvalidHeader"),
        }
    }
}