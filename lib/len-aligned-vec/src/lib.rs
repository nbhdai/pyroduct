/// # Memory Layout & Header Protocol
///
/// `LenAlignedVec` utilizes a custom 16-byte aligned memory layout compatible with FFI
/// boundary crossing. The allocation consists of a **16-byte Header** followed immediately
/// by the **Data Payload**.
///
/// ## Layout Diagram
///
/// ```text
///  Pointer (16-byte aligned)
///  │
///  ▼
/// ┌─────────────┬─────────────┬─────────────┬──────────────┬─────────────┐
/// │ Magic (u32) │  Len (u32)  │  Cap (u32)  | Version(u16) │ Status(u16) │  <-- Header (16 bytes)
/// ├─────────────┴─────────────┴─────────────┴──────────────┴─────────────┤
/// │                                                                      │
/// │                             Data Payload ...                         │  <-- Body (Len bytes)
/// │                                                                      │
/// └──────────────────────────────────────────────────────────────────────┘
/// ```
///
/// ## Header Fields
///
/// | Offset | Type  | Field   | Description |
/// |--------|-------|---------|-------------|
/// | `0x00` | `u32` | Magic   | Constant `0x7079726F` (ASCII "pyro"). Verifies pointer validity. |
/// | `0x04` | `u32` | Len     | Current length of the data payload in bytes. |
/// | `0x08` | `u32` | Cap     | Total allocated capacity (including header) in bytes. |
/// | `0x0C` | `u16` | Version | Protocol Version number |
/// | `0x0C` | `u16` | Status  | **Message Protocol Status**. Used to indicate the type of payload. |
///
/// ## Status Codes (Offset 0x0C)
///
/// When passing `Result<T, E>` across FFI or transport boundaries, the status field determines how
/// the payload should be interpreted:
///
/// * **`0` (ValidData)**: The payload is a valid `rkyv` archived `T`. Corresponds to `Ok(T)`.
/// * **`1` (UserError)**: The payload is a valid `rkyv` archived `E`. Corresponds to `Err(E)`.
/// * **`2` (Transport Error)**: The payload is a serialized `RkyvFfiError`, or a transport error. Indicates a system failure (e.g., serialization panic, validation failure) rather than a logic error.
/// * **`3` (Utf8Error)**: The payload is a raw UTF-8 string. Used as a catastrophic fallback if system error serialization fails.
/// * **`4` (ValidUtf8)**: Reserved/Unused.
use std::alloc::{self, Layout};
use std::hash::Hasher;
use std::ptr::{self, NonNull};
use std::{fmt, slice};
use std::ops::{Deref, DerefMut};

pub mod rkyv;
pub mod tokio;
pub mod ffi;

/// A 16-byte aligned buffer with a self-describing header.
/// Compatible with FFI passing as a raw pointer or TCP/Unix framing.
pub struct LenAlignedVec {
    ptr: NonNull<u8>,
}

impl LenAlignedVec {
    const ALIGN: usize = 16;
    pub const HEADER_SIZE: usize = 16;
    
    // Header Offsets
    const OFFSET_MAGIC: usize = 0;
    const OFFSET_LEN: usize = 4;
    const OFFSET_CAP: usize = 8;
    const OFFSET_VERSION: usize = 12;
    const OFFSET_STATUS: usize = 14;
    
    // Constants
    pub(crate) const MAGIC_VAL: u32 = 0x7079726F; // "pyro"
    const PROTOCOL_VERSION: u16 = 1;

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
            ptr::write(raw.add(Self::OFFSET_MAGIC) as *mut u32, Self::MAGIC_VAL);
            ptr::write(raw.add(Self::OFFSET_LEN) as *mut u32, 0); 
            ptr::write(raw.add(Self::OFFSET_CAP) as *mut u32, total_cap as u32);
            ptr::write(raw.add(Self::OFFSET_VERSION) as *mut u16, Self::PROTOCOL_VERSION);
            ptr::write(raw.add(Self::OFFSET_STATUS) as *mut u16, 0); // Default: ValidData
            
            NonNull::new_unchecked(raw)
        };

        Self { ptr }
    }

    /// Reconstructs the Vec from a raw pointer.
    pub unsafe fn from_raw(ptr: *const u8) -> Result<Self, &'static str> {
        if ptr.is_null() {
            return Err("Pointer is null");
        }
        
        if (ptr as usize) % Self::ALIGN != 0 {
            return Err("Pointer is not 16-byte aligned");
        }

        // Verify Magic Number
        let magic = unsafe { ptr::read(ptr.add(Self::OFFSET_MAGIC) as *const u32) };
        if magic != Self::MAGIC_VAL {
            return Err("Invalid magic header");
        }

        Ok(Self {
            ptr: unsafe { NonNull::new_unchecked(ptr as *mut u8) },
        })
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
    pub fn version(&self) -> u16 {
        unsafe { ptr::read(self.ptr.as_ptr().add(Self::OFFSET_VERSION) as *const u16) }
    }

    #[inline]
    pub fn set_version(&mut self, version: u16) {
        unsafe { ptr::write(self.ptr.as_ptr().add(Self::OFFSET_VERSION) as *mut u16, version) }
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
        
        let required_cap = current_len + Self::HEADER_SIZE + additional;
        let mut new_cap = current_cap * 2;
        if new_cap < required_cap {
            new_cap = required_cap;
        }
        
        let remainder = new_cap % Self::ALIGN;
        if remainder != 0 {
            new_cap += Self::ALIGN - remainder;
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


impl Clone for LenAlignedVec {
    fn clone(&self) -> Self {
        let mut new_vec = Self::with_capacity(self.len());
        
        new_vec.extend_from_slice(self.as_slice());

        new_vec.set_status(self.status());
        new_vec.set_version(self.version());

        new_vec
    }
}

impl fmt::Debug for LenAlignedVec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LenAlignedVec")
         .field("len", &self.len())
         .field("capacity", &self.capacity())
         .field("status", &self.status())
         .field("version", &self.version())
         .field("data", &self.as_slice())
         .finish()
    }
}

impl PartialEq for LenAlignedVec {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl Eq for LenAlignedVec {}

impl std::hash::Hash for LenAlignedVec {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_slice().hash(state);
    }
}

impl Deref for LenAlignedVec {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl DerefMut for LenAlignedVec {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}

impl Drop for LenAlignedVec {
    fn drop(&mut self) {
        let cap = self.capacity();
        let layout = Layout::from_size_align(cap, Self::ALIGN).unwrap();
        unsafe {
            alloc::dealloc(self.ptr.as_ptr(), layout);
        }
    }
}