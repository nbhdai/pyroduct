//! # Len Aligned Vec
//! 
//! A buffer type for rkyv transport.
//! 
//! 
//! # Memory Layout & Header Protocol
//!
//! `BridgeVec` utilizes a custom 16-byte aligned memory layout compatible with FFI
//! boundary crossing. The allocation consists of a **16-byte Header** followed immediately
//! by the **Data Payload**.
//!
//! ## Layout Diagram
//!
//! ```text
//!  Pointer (16-byte aligned)
//!  │
//!  ▼
//! ┌─────────────────┬──────────────────┬───────────────┐
//! │   Magic (u32)   │    Len (u32)     │   Cap (u32)   │ Row 1
//! ├─────────────────┼──────────────────┼───────────────┤
//! │ Wire Format(u8) │ User wire_format(u8) │  Status (u8)  │ Row 2
//! ├─────────────────┴──────────────────┴───────────────┤
//! │                                                    │
//! │               Data Payload ...                     │ <-- Body (Len bytes)
//! │                                                    │
//! └────────────────────────────────────────────────────┘
//! ```
//!
//! ## Header Fields
//!
//! | Offset | Type  | Field   | Description                                                        |
//! |--------|-------|---------|--------------------------------------------------------------------|
//! | `0x00` | `u32` | Magic   | Constant `0x7079726F` (ASCII "pyro"). Verifies pointer validity.   |
//! | `0x04` | `u32` | Len     | Current length of the data payload in bytes.                       |
//! | `0x08` | `u32` | Cap     | Total allocated capacity (including header) in bytes.              |
//! | `0x0C` | `u8`  | Format  | Protocol Version number                                            |
//! | `0x0C` | `u8`  | User    | User Version number                                                |
//! | `0x0E` | `u16` | Status  | **Message Protocol Status**. Used to indicate the type of payload. |
//!
//! ## Status Codes (Offset 0x0E)
//!
//! When passing `Result<T, E>` across FFI or transport boundaries, the status field determines how
//! the payload should be interpreted:
//!
//! * **`0` (ValidData)**: The payload is a valid `rkyv` archived `T`. Corresponds to `Ok(T)`.
//! * **`1` (UserError)**: The payload is a valid `rkyv` archived `E`. Corresponds to `Err(E)`.
//! * **`2` (Transport Error)**: The payload is a serialized `RkyvFfiError`, or a transport error. Indicates a system failure (e.g., serialization panic, validation failure) rather than a logic error.
//! * **`3` (Utf8Error)**: The payload is a raw UTF-8 string. Used as a catastrophic fallback if system error serialization fails.
//! * **`4` (ValidUtf8)**: Reserved/Unused.
use std::alloc::{self, Layout};
use std::hash::Hasher;
use std::marker::PhantomData;
use std::ptr::{self, NonNull};
use std::{fmt, slice};
use std::ops::{Deref, DerefMut};

use crate::ser_de::TypedBuf;

pub(crate) const MAGIC_VAL: u32 = 0x7079726F; // "pyro"
pub const PROTOCOL_VERSION: u8 = 1;

pub use rkyv;

mod common;
pub mod ser_de;
pub mod tokio;
pub mod ffi;

pub use bridge_derive::bridgeable;

pub trait Bridgeable: ::rkyv::Archive + Sized {
    fn serialize(&self) -> Result<BridgeVec, ::rkyv::rancor::Error>;
    fn parse(vec: BridgeVec) -> Result<TypedBuf<Self>, ::rkyv::rancor::Error>;
    fn deserialize(vec: TypedBuf<Self>) -> Result<Self, ::rkyv::rancor::Error>;
}

/// A 16-byte aligned buffer with a self-describing header.
/// Compatible with FFI passing as a raw pointer or TCP/Unix framing.
pub struct BridgeVec {
    ptr: NonNull<u8>,
}

// SAFETY: BridgeVec owns its allocation exclusively
// and contains no references to thread-local state
unsafe impl Send for BridgeVec {}

/// A borrowed, non-owning view into a BridgeVec buffer.
/// Does not free memory on drop.
pub struct BridgeVecRef<'a> {
    ptr: *const u8,
    _marker: PhantomData<&'a [u8]>,
}

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataStatus {
    ValidData = 0,
    UserError = 1,
    TransportError = 2,
    Utf8Error = 3,
    ValidUtf8 = 4,
}

impl BridgeVec {
    const ALIGN: usize = 16;
    pub const HEADER_SIZE: usize = 16;
    
    // Header Offsets
    const OFFSET_MAGIC: usize = 0;
    const OFFSET_LEN: usize = 4;
    const OFFSET_CAP: usize = 8;
    const OFFSET_WIRE_FORMAT: usize = 12;
    const OFFSET_USER_VERSION: usize = 13;
    const OFFSET_STATUS: usize = 14;

    /// Creates a new vector with a specific capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        let total_cap = (capacity + Self::HEADER_SIZE).max(Self::ALIGN);
        
        let layout = Layout::from_size_align(total_cap, Self::ALIGN)
            .expect("Invalid layout alignment");

        let ptr = unsafe {
            let raw = alloc::alloc(layout);
            if raw.is_null() {
                alloc::handle_alloc_error(layout);
            }
            
            // Initialize Header
            ptr::write(raw.add(Self::OFFSET_MAGIC) as *mut u32, MAGIC_VAL);
            ptr::write(raw.add(Self::OFFSET_LEN) as *mut u32, 0); 
            ptr::write(raw.add(Self::OFFSET_CAP) as *mut u32, total_cap as u32);
            ptr::write(raw.add(Self::OFFSET_WIRE_FORMAT) as *mut u8, PROTOCOL_VERSION);
            ptr::write(raw.add(Self::OFFSET_STATUS) as *mut u16, 0); // Default: ValidData
            
            NonNull::new_unchecked(raw)
        };

        Self { ptr }
    }

    /// Reconstructs an owned Vec from a raw pointer.
    /// 
    /// # Safety
    /// - `ptr` must have been created by `BridgeVec::into_raw()` or equivalent
    /// - Caller must ensure no other owner exists for this allocation
    /// - Caller transfers ownership to the returned `BridgeVec`
    pub unsafe fn from_raw(ptr: *const u8) -> Result<Self, &'static str> {
        if ptr.is_null() {
            return Err("Pointer is null");
        }
        
        if (ptr as usize) % Self::ALIGN != 0 {
            return Err("Pointer is not 16-byte aligned");
        }

        // Verify Magic Number
        let magic = unsafe { ptr::read(ptr.add(Self::OFFSET_MAGIC) as *const u32) };
        if magic != MAGIC_VAL {
            return Err("Invalid magic header");
        }

        Ok(Self {
            ptr: unsafe { NonNull::new_unchecked(ptr as *mut u8) },
        })
    }

    /// Creates a non-owning borrowed view from a raw pointer.
    /// 
    /// # Safety
    /// - `ptr` must point to a valid BridgeVec allocation
    /// - The allocation must remain valid for lifetime `'a`
    /// - Caller must not free or reallocate the memory during `'a`
    pub unsafe fn borrow_raw<'a>(ptr: *const u8) -> Result<BridgeVecRef<'a>, &'static str> {
        if ptr.is_null() {
            return Err("Pointer is null");
        }
        
        if (ptr as usize) % Self::ALIGN != 0 {
            return Err("Pointer is not 16-byte aligned");
        }

        let magic = unsafe { ptr::read(ptr.add(Self::OFFSET_MAGIC) as *const u32) };
        if magic != MAGIC_VAL {
            return Err("Invalid magic header");
        }

        Ok(BridgeVecRef {
            ptr,
            _marker: PhantomData,
        })
    }

    /// Consumes self and returns the raw pointer.
    /// 
    /// Caller is responsible for eventually reconstructing via `from_raw`
    /// and dropping, or manually deallocating with the correct layout.
    pub fn into_raw(self) -> *mut u8 {
        let ptr = self.ptr.as_ptr();
        std::mem::forget(self);
        ptr
    }

    // --- Header Accessors ---

    /// Gets the status code from the header (Offset 14).
    #[inline]
    pub fn status(&self) -> u16 {
        unsafe { ptr::read(self.ptr.as_ptr().add(Self::OFFSET_STATUS) as *const u16) }
    }

    /// Sets the status code in the header (Offset 14).
    /// Used by FFI and Transport layers to indicate Data vs Error.
    #[inline]
    pub fn set_status(&mut self, status: u16) {
        unsafe { ptr::write(self.ptr.as_ptr().add(Self::OFFSET_STATUS) as *mut u16, status) }
    }

    #[inline]
    pub fn version(&self) -> u8 {
        unsafe { ptr::read(self.ptr.as_ptr().add(Self::OFFSET_USER_VERSION) as *const u8) }
    }

    #[inline]
    pub fn set_version(&mut self, version: u8) {
        unsafe { ptr::write(self.ptr.as_ptr().add(Self::OFFSET_USER_VERSION) as *mut u8, version) }
    }

    #[inline]
    pub fn wire_format(&self) -> u8 {
        unsafe { ptr::read(self.ptr.as_ptr().add(Self::OFFSET_WIRE_FORMAT) as *const u8) }
    }

    #[doc(hidden)]
    #[inline]
    pub fn set_wire_format(&mut self, version: u8) {
        unsafe { ptr::write(self.ptr.as_ptr().add(Self::OFFSET_WIRE_FORMAT) as *mut u8, version) }
    }

    // --- Data Accessors ---

    pub fn as_ptr(&self) -> *const u8 {
        self.ptr.as_ptr()
    }

    pub fn data_ptr(&self) -> *const u8 {
        unsafe { self.ptr.as_ptr().add(Self::HEADER_SIZE) }
    }

    #[inline]
    pub fn len(&self) -> usize {
        unsafe {
            ptr::read(self.ptr.as_ptr().add(Self::OFFSET_LEN) as *const u32) as usize
        }
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        unsafe {
            ptr::read(self.ptr.as_ptr().add(Self::OFFSET_CAP) as *const u32) as usize
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns a slice containing the Header (16 bytes) AND the Data (len bytes).
    /// Useful for zero-copy writing to streams.
    pub fn as_packet_slice(&self) -> &[u8] {
        unsafe {
            slice::from_raw_parts(self.ptr.as_ptr(), Self::HEADER_SIZE + self.len())
        }
    }

    // --- Vec Operations ---

    pub fn push(&mut self, byte: u8) {
        if self.len() + Self::HEADER_SIZE == self.capacity() {
            self.grow(1);
        }

        unsafe {
            let len = self.len();
            let data_start = self.ptr.as_ptr().add(Self::HEADER_SIZE);
            ptr::write(data_start.add(len), byte);
            self.set_len(len + 1);
        }
    }

    pub fn extend_from_slice(&mut self, other: &[u8]) {
        let required = other.len();
        let current_len = self.len();
        let current_cap = self.capacity();

        if current_len + required + Self::HEADER_SIZE > current_cap {
            self.grow(required);
        }

        unsafe {
            ptr::copy_nonoverlapping(
                other.as_ptr(),
                self.ptr.as_ptr().add(Self::HEADER_SIZE + current_len),
                required,
            );
            self.set_len(current_len + required);
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        unsafe {
            slice::from_raw_parts(self.data_ptr(), self.len())
        }
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe {
            slice::from_raw_parts_mut(self.ptr.as_ptr().add(Self::HEADER_SIZE), self.len())
        }
    }

    pub fn clear(&mut self) {
        unsafe { self.set_len(0); }
    }

    // --- Internals ---

    #[inline]
    unsafe fn set_len(&mut self, new_len: usize) {
        unsafe { ptr::write(self.ptr.as_ptr().add(Self::OFFSET_LEN) as *mut u32, new_len as u32) };
    }

    fn grow(&mut self, additional: usize) {
        let current_cap = self.capacity();
        let current_len = self.len();
        
        let required_cap = current_len
            .checked_add(Self::HEADER_SIZE)
            .and_then(|v| v.checked_add(additional))
            .expect("capacity overflow");
        
        let mut new_cap = current_cap.saturating_mul(2).max(required_cap);
        
        let remainder = new_cap % Self::ALIGN;
        if remainder != 0 {
            new_cap = new_cap
                .checked_add(Self::ALIGN - remainder)
                .expect("capacity overflow during alignment");
        }

        let old_layout = Layout::from_size_align(current_cap, Self::ALIGN).unwrap();
        
        unsafe {
            let new_ptr = alloc::realloc(self.ptr.as_ptr(), old_layout, new_cap);
            if new_ptr.is_null() {
                alloc::handle_alloc_error(Layout::from_size_align(new_cap, Self::ALIGN).unwrap());
            }
            ptr::write(new_ptr.add(Self::OFFSET_CAP) as *mut u32, new_cap as u32);
            self.ptr = NonNull::new_unchecked(new_ptr);
        }
    }
}

// --- BridgeVecRef Implementation ---

impl<'a> BridgeVecRef<'a> {
    #[inline]
    pub fn status(&self) -> u16 {
        unsafe { ptr::read(self.ptr.add(BridgeVec::OFFSET_STATUS) as *const u16) }
    }

    #[inline]
    pub fn wire_format(&self) -> u16 {
        unsafe { ptr::read(self.ptr.add(BridgeVec::OFFSET_WIRE_FORMAT) as *const u16) }
    }

    #[inline]
    pub fn len(&self) -> usize {
        unsafe { ptr::read(self.ptr.add(BridgeVec::OFFSET_LEN) as *const u32) as usize }
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        unsafe { ptr::read(self.ptr.add(BridgeVec::OFFSET_CAP) as *const u32) as usize }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn as_ptr(&self) -> *const u8 {
        self.ptr
    }

    pub fn data_ptr(&self) -> *const u8 {
        unsafe { self.ptr.add(BridgeVec::HEADER_SIZE) }
    }

    pub fn as_slice(&self) -> &'a [u8] {
        unsafe { slice::from_raw_parts(self.data_ptr(), self.len()) }
    }

    pub fn as_packet_slice(&self) -> &'a [u8] {
        unsafe { slice::from_raw_parts(self.ptr, BridgeVec::HEADER_SIZE + self.len()) }
    }
}

impl<'a> fmt::Debug for BridgeVecRef<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BridgeVecRef")
         .field("len", &self.len())
         .field("capacity", &self.capacity())
         .field("status", &self.status())
         .field("version", &self.wire_format())
         .field("data", &self.as_slice())
         .finish()
    }
}

// --- BridgeVec Trait Implementations ---

impl Clone for BridgeVec {
    fn clone(&self) -> Self {
        let mut new_vec = Self::with_capacity(self.len());
        
        new_vec.extend_from_slice(self.as_slice());

        new_vec.set_status(self.status());
        new_vec.set_wire_format(self.wire_format());

        new_vec
    }
}

impl fmt::Debug for BridgeVec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BridgeVec")
         .field("len", &self.len())
         .field("capacity", &self.capacity())
         .field("status", &self.status())
         .field("version", &self.wire_format())
         .field("data", &self.as_slice())
         .finish()
    }
}

impl PartialEq for BridgeVec {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl Eq for BridgeVec {}

impl std::hash::Hash for BridgeVec {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_slice().hash(state);
    }
}

impl Deref for BridgeVec {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl DerefMut for BridgeVec {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}

impl Drop for BridgeVec {
    fn drop(&mut self) {
        let cap = self.capacity();
        let layout = Layout::from_size_align(cap, Self::ALIGN).unwrap();
        unsafe {
            alloc::dealloc(self.ptr.as_ptr(), layout);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::alloc::{Layout, alloc, dealloc};
use std::ptr;

// =============================================================================
// Construction & Basic Properties
// =============================================================================

#[test]
fn test_with_capacity_zero() {
    let vec = BridgeVec::with_capacity(0);
    assert_eq!(vec.len(), 0);
    assert!(vec.is_empty());
    // Minimum allocation is ALIGN (16), so capacity >= 16
    assert!(vec.capacity() >= BridgeVec::HEADER_SIZE);
}

#[test]
fn test_with_capacity_small() {
    let vec = BridgeVec::with_capacity(10);
    assert_eq!(vec.len(), 0);
    assert!(vec.capacity() >= 10 + BridgeVec::HEADER_SIZE);
}

#[test]
fn test_with_capacity_large() {
    let vec = BridgeVec::with_capacity(10000);
    assert_eq!(vec.len(), 0);
    assert!(vec.capacity() >= 10000 + BridgeVec::HEADER_SIZE);
}

#[test]
fn test_default_header_values() {
    let vec = BridgeVec::with_capacity(10);
    assert_eq!(vec.status(), 0);
    assert_eq!(vec.wire_format(), 1);
}

// =============================================================================
// Alignment & Layout
// =============================================================================

#[test]
fn test_base_pointer_alignment() {
    let vec = BridgeVec::with_capacity(100);
    let addr = vec.as_ptr() as usize;
    assert_eq!(addr % 16, 0, "Base pointer must be 16-byte aligned");
}

#[test]
fn test_data_pointer_alignment() {
    let vec = BridgeVec::with_capacity(100);
    let base_addr = vec.as_ptr() as usize;
    let data_addr = vec.data_ptr() as usize;
    
    assert_eq!(data_addr % 16, 0, "Data pointer must be 16-byte aligned");
    assert_eq!(data_addr - base_addr, 16, "Header size must be exactly 16 bytes");
}

#[test]
fn test_alignment_preserved_after_grow() {
    let mut vec = BridgeVec::with_capacity(10);
    
    // Force multiple reallocations
    for i in 0..1000 {
        vec.push(i as u8);
    }
    
    let addr = vec.as_ptr() as usize;
    assert_eq!(addr % 16, 0, "Alignment must be preserved after realloc");
}

// =============================================================================
// Header Accessors
// =============================================================================

#[test]
fn test_status_read_write() {
    let mut vec = BridgeVec::with_capacity(10);
    
    assert_eq!(vec.status(), 0);
    
    vec.set_status(42);
    assert_eq!(vec.status(), 42);
    
    vec.set_status(0xFFFF);
    assert_eq!(vec.status(), 0xFFFF);
    
    vec.set_status(0);
    assert_eq!(vec.status(), 0);
}

#[test]
fn test_version_read_write() {
    let mut vec = BridgeVec::with_capacity(10);

    assert_eq!(vec.wire_format(), 1);
    assert_eq!(vec.version(), 0);
    
    vec.set_wire_format(99);
    assert_eq!(vec.wire_format(), 99);
    vec.set_version(99);
    assert_eq!(vec.version(), 99);
    
    vec.set_wire_format(0xAB);
    assert_eq!(vec.wire_format(), 0xAB);
    vec.set_version(0xFF);
    assert_eq!(vec.version(), 0xFF);
}

#[test]
fn test_header_fields_independent() {
    let mut vec = BridgeVec::with_capacity(10);
    
    vec.set_status(0x1234);
    vec.set_wire_format(0x56);
    vec.set_version(0x78);
    
    // Verify they don't interfere with each other
    assert_eq!(vec.status(), 0x1234);
    assert_eq!(vec.wire_format(), 0x56);
    assert_eq!(vec.version(), 0x78);
    
    vec.set_status(0xAAAA);
    assert_eq!(vec.status(), 0xAAAA);
    assert_eq!(vec.wire_format(), 0x56); // Unchanged
    assert_eq!(vec.version(), 0x78);
}

// =============================================================================
// Data Operations - push
// =============================================================================

#[test]
fn test_push_single() {
    let mut vec = BridgeVec::with_capacity(10);
    vec.push(0xAB);
    
    assert_eq!(vec.len(), 1);
    assert_eq!(vec.as_slice(), &[0xAB]);
}

#[test]
fn test_push_multiple() {
    let mut vec = BridgeVec::with_capacity(10);
    
    for i in 0..5 {
        vec.push(i);
    }
    
    assert_eq!(vec.len(), 5);
    assert_eq!(vec.as_slice(), &[0, 1, 2, 3, 4]);
}

#[test]
fn test_push_triggers_grow() {
    let mut vec = BridgeVec::with_capacity(2);
    let initial_cap = vec.capacity();
    
    // Push more than initial capacity
    for i in 0..100 {
        vec.push(i as u8);
    }
    
    assert_eq!(vec.len(), 100);
    assert!(vec.capacity() > initial_cap);
    
    // Verify data integrity
    for i in 0..100 {
        assert_eq!(vec.as_slice()[i], i as u8);
    }
}

// =============================================================================
// Data Operations - extend_from_slice
// =============================================================================

#[test]
fn test_extend_from_slice_empty() {
    let mut vec = BridgeVec::with_capacity(10);
    vec.extend_from_slice(&[]);
    
    assert_eq!(vec.len(), 0);
    assert!(vec.is_empty());
}

#[test]
fn test_extend_from_slice_small() {
    let mut vec = BridgeVec::with_capacity(10);
    vec.extend_from_slice(&[1, 2, 3, 4, 5]);
    
    assert_eq!(vec.len(), 5);
    assert_eq!(vec.as_slice(), &[1, 2, 3, 4, 5]);
}

#[test]
fn test_extend_from_slice_multiple() {
    let mut vec = BridgeVec::with_capacity(10);
    
    vec.extend_from_slice(&[1, 2, 3]);
    vec.extend_from_slice(&[4, 5, 6]);
    vec.extend_from_slice(&[7, 8, 9]);
    
    assert_eq!(vec.len(), 9);
    assert_eq!(vec.as_slice(), &[1, 2, 3, 4, 5, 6, 7, 8, 9]);
}

#[test]
fn test_extend_from_slice_triggers_grow() {
    let mut vec = BridgeVec::with_capacity(5);
    let pattern: Vec<u8> = (0..200).collect();
    
    vec.extend_from_slice(&pattern);
    
    assert_eq!(vec.len(), 200);
    assert_eq!(vec.as_slice(), &pattern[..]);
}

// =============================================================================
// Data Operations - clear
// =============================================================================

#[test]
fn test_clear_empty() {
    let mut vec = BridgeVec::with_capacity(10);
    vec.clear();
    
    assert_eq!(vec.len(), 0);
    assert!(vec.is_empty());
}

#[test]
fn test_clear_with_data() {
    let mut vec = BridgeVec::with_capacity(10);
    vec.extend_from_slice(&[1, 2, 3, 4, 5]);
    
    let cap_before = vec.capacity();
    vec.clear();
    
    assert_eq!(vec.len(), 0);
    assert!(vec.is_empty());
    assert_eq!(vec.capacity(), cap_before); // Capacity unchanged
}

#[test]
fn test_clear_then_reuse() {
    let mut vec = BridgeVec::with_capacity(10);
    
    vec.extend_from_slice(&[1, 2, 3]);
    vec.clear();
    vec.extend_from_slice(&[4, 5, 6, 7]);
    
    assert_eq!(vec.len(), 4);
    assert_eq!(vec.as_slice(), &[4, 5, 6, 7]);
}

// =============================================================================
// Slice Access
// =============================================================================

#[test]
fn test_as_slice_empty() {
    let vec = BridgeVec::with_capacity(10);
    assert_eq!(vec.as_slice(), &[]);
}

#[test]
fn test_as_slice_with_data() {
    let mut vec = BridgeVec::with_capacity(10);
    vec.extend_from_slice(b"hello");
    
    assert_eq!(vec.as_slice(), b"hello");
}

#[test]
fn test_as_mut_slice() {
    let mut vec = BridgeVec::with_capacity(10);
    vec.extend_from_slice(&[1, 2, 3, 4, 5]);
    
    let slice = vec.as_mut_slice();
    slice[0] = 100;
    slice[4] = 200;
    
    assert_eq!(vec.as_slice(), &[100, 2, 3, 4, 200]);
}

#[test]
fn test_as_packet_slice() {
    let mut vec = BridgeVec::with_capacity(10);
    vec.extend_from_slice(&[0xAA, 0xBB, 0xCC]);
    
    let packet = vec.as_packet_slice();
    
    // Should be header (16 bytes) + data (3 bytes)
    assert_eq!(packet.len(), 16 + 3);
    
    // Verify magic at start
    let magic = u32::from_ne_bytes(packet[0..4].try_into().unwrap());
    assert_eq!(magic, 0x7079726F);
    
    // Verify data at end
    assert_eq!(&packet[16..], &[0xAA, 0xBB, 0xCC]);
}

// =============================================================================
// Deref / DerefMut
// =============================================================================

#[test]
fn test_deref() {
    let mut vec = BridgeVec::with_capacity(10);
    vec.extend_from_slice(b"test");
    
    let slice: &[u8] = &vec;
    assert_eq!(slice, b"test");
}

#[test]
fn test_deref_mut() {
    let mut vec = BridgeVec::with_capacity(10);
    vec.extend_from_slice(&[1, 2, 3]);
    
    let slice: &mut [u8] = &mut vec;
    slice[1] = 99;
    
    assert_eq!(vec.as_slice(), &[1, 99, 3]);
}

// =============================================================================
// Clone
// =============================================================================

#[test]
fn test_clone_empty() {
    let original = BridgeVec::with_capacity(10);
    let cloned = original.clone();
    
    assert_eq!(cloned.len(), 0);
    assert_eq!(cloned.status(), original.status());
    assert_eq!(cloned.wire_format(), original.wire_format());
}

#[test]
fn test_clone_with_data() {
    let mut original = BridgeVec::with_capacity(10);
    original.extend_from_slice(b"hello world");
    original.set_status(42);
    original.set_wire_format(7);
    
    let cloned = original.clone();
    
    assert_eq!(cloned.as_slice(), b"hello world");
    assert_eq!(cloned.status(), 42);
    assert_eq!(cloned.wire_format(), 7);
    
    // Verify independence
    assert_ne!(original.as_ptr(), cloned.as_ptr());
}

#[test]
fn test_clone_independence() {
    let mut original = BridgeVec::with_capacity(10);
    original.extend_from_slice(&[1, 2, 3]);
    
    let mut cloned = original.clone();
    cloned.extend_from_slice(&[4, 5, 6]);
    cloned.set_status(99);
    
    // Original unchanged
    assert_eq!(original.as_slice(), &[1, 2, 3]);
    assert_eq!(original.status(), 0);
    
    // Clone modified
    assert_eq!(cloned.as_slice(), &[1, 2, 3, 4, 5, 6]);
    assert_eq!(cloned.status(), 99);
}

// =============================================================================
// PartialEq / Eq / Hash
// =============================================================================

#[test]
fn test_eq_empty() {
    let a = BridgeVec::with_capacity(10);
    let b = BridgeVec::with_capacity(20);
    
    assert_eq!(a, b);
}

#[test]
fn test_eq_same_data() {
    let mut a = BridgeVec::with_capacity(10);
    let mut b = BridgeVec::with_capacity(20);
    
    a.extend_from_slice(b"test");
    b.extend_from_slice(b"test");
    
    assert_eq!(a, b);
}

#[test]
fn test_eq_different_data() {
    let mut a = BridgeVec::with_capacity(10);
    let mut b = BridgeVec::with_capacity(10);
    
    a.extend_from_slice(b"hello");
    b.extend_from_slice(b"world");
    
    assert_ne!(a, b);
}

#[test]
fn test_eq_ignores_status() {
    let mut a = BridgeVec::with_capacity(10);
    let mut b = BridgeVec::with_capacity(10);
    
    a.extend_from_slice(b"test");
    b.extend_from_slice(b"test");
    
    a.set_status(1);
    b.set_status(2);
    
    // Equality is based on data only
    assert_eq!(a, b);
}

#[test]
fn test_hash_consistency() {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut a = BridgeVec::with_capacity(10);
    let mut b = BridgeVec::with_capacity(20);
    
    a.extend_from_slice(b"test data");
    b.extend_from_slice(b"test data");
    
    let hash_a = {
        let mut h = DefaultHasher::new();
        a.hash(&mut h);
        h.finish()
    };
    
    let hash_b = {
        let mut h = DefaultHasher::new();
        b.hash(&mut h);
        h.finish()
    };
    
    assert_eq!(hash_a, hash_b);
}

// =============================================================================
// Debug
// =============================================================================

#[test]
fn test_debug_format() {
    let mut vec = BridgeVec::with_capacity(10);
    vec.extend_from_slice(&[1, 2, 3]);
    vec.set_status(5);
    
    let debug = format!("{:?}", vec);
    
    assert!(debug.contains("BridgeVec"));
    assert!(debug.contains("len: 3"));
    assert!(debug.contains("status: 5"));
}

// =============================================================================
// from_raw
// =============================================================================

#[test]
fn test_from_raw_null() {
    let result = unsafe { BridgeVec::from_raw(std::ptr::null()) };
    assert!(result.is_err());
    assert_eq!(result.err(), Some("Pointer is null"));
}

#[test]
fn test_from_raw_misaligned() {
    let layout = Layout::from_size_align(64, 16).unwrap();
    let ptr = unsafe { alloc(layout) };
    
    let bad_ptr = unsafe { ptr.add(1) };
    let result = unsafe { BridgeVec::from_raw(bad_ptr) };
    
    assert!(result.is_err());
    assert_eq!(result.err(), Some("Pointer is not 16-byte aligned"));
    
    unsafe { dealloc(ptr, layout); }
}

#[test]
fn test_from_raw_bad_magic() {
    let layout = Layout::from_size_align(32, 16).unwrap();
    let ptr = unsafe { alloc(layout) };
    
    unsafe {
        ptr::write(ptr as *mut u32, 0xDEADBEEF);
    }
    
    let result = unsafe { BridgeVec::from_raw(ptr) };
    
    assert!(result.is_err());
    assert_eq!(result.err(), Some("Invalid magic header"));
    
    unsafe { dealloc(ptr, layout); }
}

#[test]
fn test_from_raw_valid() {
    let mut original = BridgeVec::with_capacity(50);
    original.extend_from_slice(&[0xAA, 0xBB, 0xCC]);
    original.set_status(7);
    
    let raw_ptr = original.into_raw();
    
    let reconstructed = unsafe { 
        BridgeVec::from_raw(raw_ptr).expect("Should reconstruct from valid ptr") 
    };
    
    assert_eq!(reconstructed.len(), 3);
    assert_eq!(reconstructed.status(), 7);
    assert_eq!(reconstructed.as_slice(), &[0xAA, 0xBB, 0xCC]);
}

// =============================================================================
// into_raw
// =============================================================================

#[test]
fn test_into_raw_ownership_transfer() {
    let mut vec = BridgeVec::with_capacity(10);
    vec.extend_from_slice(b"test");
    vec.set_status(42);
    
    let ptr = vec.into_raw();
    // vec is now consumed
    
    let recovered = unsafe { BridgeVec::from_raw(ptr).unwrap() };
    assert_eq!(recovered.as_slice(), b"test");
    assert_eq!(recovered.status(), 42);
}

#[test]
fn test_into_raw_roundtrip_preserves_all() {
    let mut vec = BridgeVec::with_capacity(100);
    vec.extend_from_slice(b"roundtrip test data");
    vec.set_status(0x1234);
    vec.set_wire_format(0x56);
    
    let ptr = vec.into_raw();
    let recovered = unsafe { BridgeVec::from_raw(ptr).unwrap() };
    
    assert_eq!(recovered.as_slice(), b"roundtrip test data");
    assert_eq!(recovered.status(), 0x1234);
    assert_eq!(recovered.wire_format(), 0x56);
}

// =============================================================================
// borrow_raw / BridgeVecRef
// =============================================================================

#[test]
fn test_borrow_raw_null() {
    let result = unsafe { BridgeVec::borrow_raw(std::ptr::null()) };
    assert!(result.is_err());
    assert_eq!(result.err(), Some("Pointer is null"));
}

#[test]
fn test_borrow_raw_misaligned() {
    let layout = Layout::from_size_align(64, 16).unwrap();
    let ptr = unsafe { alloc(layout) };
    
    let bad_ptr = unsafe { ptr.add(1) };
    let result = unsafe { BridgeVec::borrow_raw(bad_ptr) };
    
    assert!(result.is_err());
    assert_eq!(result.err(), Some("Pointer is not 16-byte aligned"));
    
    unsafe { dealloc(ptr, layout); }
}

#[test]
fn test_borrow_raw_bad_magic() {
    let layout = Layout::from_size_align(32, 16).unwrap();
    let ptr = unsafe { alloc(layout) };
    
    unsafe {
        ptr::write(ptr as *mut u32, 0xDEADBEEF);
    }
    
    let result = unsafe { BridgeVec::borrow_raw(ptr) };
    
    assert!(result.is_err());
    assert_eq!(result.err(), Some("Invalid magic header"));
    
    unsafe { dealloc(ptr, layout); }
}

#[test]
fn test_borrow_raw_non_owning() {
    let mut original = BridgeVec::with_capacity(50);
    original.extend_from_slice(&[0xAA, 0xBB, 0xCC]);
    original.set_status(7);
    original.set_wire_format(3);
    
    let borrowed = unsafe { 
        BridgeVec::borrow_raw(original.as_ptr()).expect("Should borrow from valid ptr") 
    };
    
    assert_eq!(borrowed.len(), 3);
    assert_eq!(borrowed.status(), 7);
    assert_eq!(borrowed.wire_format(), 3);
    assert_eq!(borrowed.as_slice(), &[0xAA, 0xBB, 0xCC]);
    
    // Original still valid
    assert_eq!(original.len(), 3);
    assert_eq!(original.as_slice(), &[0xAA, 0xBB, 0xCC]);
}

#[test]
fn test_borrow_raw_does_not_drop() {
    let mut original = BridgeVec::with_capacity(50);
    original.extend_from_slice(b"test");
    
    {
        let _borrowed = unsafe { 
            BridgeVec::borrow_raw(original.as_ptr()).unwrap() 
        };
        // borrowed goes out of scope here but should NOT free memory
    }
    
    // Original should still be valid
    assert_eq!(original.as_slice(), b"test");
}

// =============================================================================
// BridgeVecRef
// =============================================================================

#[test]
fn test_vec_ref_accessors() {
    let mut vec = BridgeVec::with_capacity(100);
    vec.extend_from_slice(b"reference test");
    vec.set_status(42);
    vec.set_wire_format(7);
    
    let borrowed = unsafe { BridgeVec::borrow_raw(vec.as_ptr()).unwrap() };
    
    assert_eq!(borrowed.len(), 14);
    assert_eq!(borrowed.capacity(), vec.capacity());
    assert_eq!(borrowed.status(), 42);
    assert_eq!(borrowed.wire_format(), 7);
    assert!(!borrowed.is_empty());
}

#[test]
fn test_vec_ref_empty() {
    let vec = BridgeVec::with_capacity(10);
    
    let borrowed = unsafe { BridgeVec::borrow_raw(vec.as_ptr()).unwrap() };
    
    assert_eq!(borrowed.len(), 0);
    assert!(borrowed.is_empty());
    assert_eq!(borrowed.as_slice(), &[]);
}

#[test]
fn test_vec_ref_as_ptr() {
    let vec = BridgeVec::with_capacity(10);
    
    let borrowed = unsafe { BridgeVec::borrow_raw(vec.as_ptr()).unwrap() };
    
    assert_eq!(borrowed.as_ptr(), vec.as_ptr());
    assert_eq!(borrowed.data_ptr(), vec.data_ptr());
}

#[test]
fn test_vec_ref_as_packet_slice() {
    let mut vec = BridgeVec::with_capacity(10);
    vec.extend_from_slice(&[1, 2, 3]);
    
    let borrowed = unsafe { BridgeVec::borrow_raw(vec.as_ptr()).unwrap() };
    
    let packet = borrowed.as_packet_slice();
    assert_eq!(packet.len(), 16 + 3);
    assert_eq!(&packet[16..], &[1, 2, 3]);
}

#[test]
fn test_vec_ref_debug() {
    let mut vec = BridgeVec::with_capacity(10);
    vec.extend_from_slice(&[1, 2, 3]);
    vec.set_status(5);
    
    let borrowed = unsafe { BridgeVec::borrow_raw(vec.as_ptr()).unwrap() };
    let debug = format!("{:?}", borrowed);
    
    assert!(debug.contains("BridgeVecRef"));
    assert!(debug.contains("len: 3"));
    assert!(debug.contains("status: 5"));
}

// =============================================================================
// Grow behavior
// =============================================================================

#[test]
fn test_grow_preserves_header() {
    let mut vec = BridgeVec::with_capacity(10);
    vec.set_status(42);
    vec.set_wire_format(99);
    
    // Force grow
    let pattern: Vec<u8> = (0..500).map(|i| i as u8).collect();
    vec.extend_from_slice(&pattern);
    
    assert_eq!(vec.status(), 42, "Status must be preserved across realloc");
    assert_eq!(vec.wire_format(), 99, "Version must be preserved across realloc");
    assert_eq!(vec.as_slice(), &pattern[..]);
}

#[test]
fn test_grow_preserves_data() {
    let mut vec = BridgeVec::with_capacity(10);
    
    for i in 0u8..=255 {
        vec.push(i);
    }
    
    // Verify all data intact
    for i in 0u8..=255 {
        assert_eq!(vec.as_slice()[i as usize], i);
    }
}

#[test]
fn test_grow_maintains_alignment() {
    let mut vec = BridgeVec::with_capacity(1);
    
    for _ in 0..10 {
        // Each iteration should trigger growth
        let current_cap = vec.capacity();
        while vec.len() + BridgeVec::HEADER_SIZE < current_cap {
            vec.push(0);
        }
        vec.push(0); // Trigger grow
        
        let addr = vec.as_ptr() as usize;
        assert_eq!(addr % 16, 0, "Must remain 16-byte aligned after grow");
    }
}

// =============================================================================
// Edge cases
// =============================================================================

#[test]
fn test_large_allocation() {
    let mut vec = BridgeVec::with_capacity(1_000_000);
    let data = vec![0xABu8; 1_000_000];
    
    vec.extend_from_slice(&data);
    
    assert_eq!(vec.len(), 1_000_000);
    assert!(vec.as_slice().iter().all(|&b| b == 0xAB));
}

#[test]
fn test_mixed_operations() {
    let mut vec = BridgeVec::with_capacity(10);
    
    vec.push(1);
    vec.extend_from_slice(&[2, 3, 4]);
    vec.push(5);
    vec.set_status(100);
    vec.extend_from_slice(&[6, 7]);
    vec.push(8);
    
    assert_eq!(vec.as_slice(), &[1, 2, 3, 4, 5, 6, 7, 8]);
    assert_eq!(vec.status(), 100);
}

#[test]
fn test_clear_and_refill_multiple_times() {
    let mut vec = BridgeVec::with_capacity(10);
    
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