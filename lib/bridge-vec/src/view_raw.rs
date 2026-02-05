use crate::{
    BridgeError, BridgeVec,
    header::{BridgeData, BridgeHeader, BridgeHeaderMut, BridgeParser, DataStatus, MutBridgeData, MAGIC_VAL, OwnedBridgeData}
};
use crate::view::{BridgeErrorView, BridgeView};
use std::{fmt::{self, Debug}, ops::{Deref, DerefMut}};

/// A mutable, zero-copy view into a BridgeVec residing in a mutable byte slice.
///
/// This struct holds a mutable reference to the exact slice of memory containing
/// the Header and the Data payload. It allows modification of both the header
/// and the payload, but cannot reallocate (grow) the underlying memory.
pub struct BridgeMutView<'a> {
    // The slice containing [Header (16 bytes) | Payload (len bytes)]
    raw_slice: &'a mut [u8],
}

impl BridgeData for BridgeMutView<'_> {
    #[inline]
    fn header(&self) -> &[u8; 16] {
        // Safety: The constructor guarantees the slice is at least 16 bytes.
        unsafe { &*(self.raw_slice.as_ptr() as *const [u8; 16]) }
    }
}

impl MutBridgeData for BridgeMutView<'_> {
    #[inline]
    fn header_mut(&mut self) -> &mut [u8; 16] {
        // Safety: The constructor guarantees the slice is at least 16 bytes.
        unsafe { &mut *(self.raw_slice.as_mut_ptr() as *mut [u8; 16]) }
    }
}

impl Deref for BridgeMutView<'_> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.raw_slice[BridgeParser::HEADER_SIZE..]
    }
}

impl DerefMut for BridgeMutView<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.raw_slice[BridgeParser::HEADER_SIZE..]
    }
}

impl fmt::Debug for BridgeMutView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BridgeMutView")
            .field("len", &self.len())
            .field("capacity", &self.capacity())
            .field("status", &self.status())
            .field("wire_fmt", &self.wire_format())
            .field("data", &self.as_slice()) // Use as_slice to avoid moving
            .finish()
    }
}

impl<'a> BridgeMutView<'a> {
    /// Returns the slice containing the Header and the Data.
    #[inline]
    pub fn inner(&self) -> &[u8] {
        self.raw_slice
    }

    /// Returns the mutable slice containing the Header and the Data.
    #[inline]
    pub fn inner_mut(&mut self) -> &mut [u8] {
        self.raw_slice
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.raw_slice[BridgeParser::HEADER_SIZE..]
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.raw_slice[BridgeParser::HEADER_SIZE..]
    }

    pub fn capacity(&self) -> usize {
        self.raw_slice.len() - BridgeParser::HEADER_SIZE
    }

    /// Downgrades this mutable view into a read-only `BridgeView`.
    ///
    /// This consumes the mutable view.
    pub fn into_view(self) -> BridgeView<'a> {
        // We use get_view with offset 0 to reconstruct the read-only view.
        // This is safe because BridgeMutView is guaranteed to be valid by its own constructor.
        // We use .expect() because validation should have already passed during creation.
        crate::view::get_view(self.raw_slice, 0).expect("BridgeMutView is always valid")
    }

    /// Updates the length of the payload in the header.
    ///
    /// # Panics
    /// Panics if `new_len` exceeds the capacity (the size of the underlying slice - header size).
    pub fn set_payload_len(&mut self, new_len: usize) {
        if new_len > self.capacity() {
            panic!("BridgeMutView: new length {} exceeds capacity {}", new_len, self.capacity());
        }
        self.set_len(new_len as u32);
    }

    /// Marks the view as valid (status = 0).
    pub fn set_valid(&mut self) {
        self.set_status(DataStatus::ValidData);
    }
}

/// Creates a `BridgeMutView` from a raw mutable memory slice and an offset.
///
/// This performs bounds checks and header validation.
///
/// # Arguments
///
/// * `wasm_memory` - The entire available memory buffer.
/// * `ptr_offset` - The index into `wasm_memory` where the BridgeVec starts.
pub fn get_view_mut<'a>(
    wasm_memory: &'a mut [u8], 
    ptr_offset: u32
) -> Result<BridgeMutView<'a>, BridgeErrorView> {
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
    // We can safely slice because of check #2.
    // We read strictly without mutation first to verify validity.
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
    Ok(BridgeMutView {
        raw_slice: &mut wasm_memory[offset..required_end],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BridgeVec;

    #[test]
    fn test_view_mut_modification() {
        // Create a real BridgeVec
        let mut bv = BridgeVec::with_capacity(32);
        bv.extend_from_slice(b"hello");
        
        // Mock "wasm memory"
        let memory = bv.as_packet_slice_mut();
        
        // Create Mutable View
        let mut view = get_view_mut(memory, 0).expect("Should create view");
        
        assert_eq!(view.len(), 5);
        assert_eq!(&*view, b"hello");
        
        // Modify Payload
        view[0] = b'H';
        assert_eq!(&*view, b"Hello");
        
        // Modify Header (increase length)
        // Note: In a real scenario, you must write data before increasing length.
        // We have capacity for 32, current len is 5.
        view.as_mut_slice()[5] = b'!';
        view.set_payload_len(6);
        
        assert_eq!(view.len(), 6);
        assert_eq!(&*view, b"Hello!");
    }
    
    #[test]
    fn test_view_mut_downgrade() {
        let mut bv = BridgeVec::with_capacity(32);
        bv.extend_from_slice(b"test");
        let memory = bv.as_packet_slice_mut();
        
        let view_mut = get_view_mut(memory, 0).expect("Should create view");
        let view = view_mut.into_view();
        
        assert_eq!(&*view, b"test");
    }
}