use std::alloc::{self, Layout};
use std::ptr::{self, NonNull};
use std::slice;
use std::ops::{Deref, DerefMut};

mod rkyv;

/// A 16-byte aligned buffer with a self-describing header.
/// Compatible with FFI passing as a raw pointer.
pub struct LenAlignedVec {
    ptr: NonNull<u8>,
}

impl LenAlignedVec {
    const ALIGN: usize = 16;
    const HEADER_SIZE: usize = 16;
    
    // Offsets for specific u32 fields
    const OFFSET_MAGIC: usize = 0;
    const OFFSET_LEN: usize = 4;
    const OFFSET_CAP: usize = 8;
    
    // 0x524B5956 is ASCII for "pyro"
    const MAGIC_VAL: u32 = 0x7079726F; 

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
            ptr::write(raw.add(Self::OFFSET_LEN) as *mut u32, 0); // Length = 0
            ptr::write(raw.add(Self::OFFSET_CAP) as *mut u32, total_cap as u32);
            // Padding at [12..16] is left uninitialized
            
            NonNull::new_unchecked(raw)
        };

        Self { ptr }
    }

    /// Reconstructs the Vec from a raw pointer.
    /// 
    /// # Safety
    /// 1. `ptr` must be non-null and 16-byte aligned.
    /// 2. `ptr` must have been allocated by `LenAlignedVec` (or compatible allocator).
    /// 3. Ownership is transferred to Rust; the memory will be deallocated when this struct drops.
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

    /// Returns the raw pointer to the start of the allocation (Header).
    pub fn as_ptr(&self) -> *const u8 {
        self.ptr.as_ptr()
    }

    /// Returns the raw pointer to the start of the data (skipping Header).
    pub fn data_ptr(&self) -> *const u8 {
        unsafe { self.ptr.as_ptr().add(Self::HEADER_SIZE) }
    }

    /// Get the length (number of data bytes) directly from the header.
    #[inline]
    pub fn len(&self) -> usize {
        unsafe {
            ptr::read(self.ptr.as_ptr().add(Self::OFFSET_LEN) as *const u32) as usize
        }
    }

    /// Get the total allocated capacity directly from the header.
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

    // --- Core Vec Functionality ---

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

    pub fn pop(&mut self) -> Option<u8> {
        let len = self.len();
        if len == 0 {
            None
        } else {
            unsafe {
                let new_len = len - 1;
                self.set_len(new_len);
                Some(ptr::read(self.ptr.as_ptr().add(Self::HEADER_SIZE + new_len)))
            }
        }
    }

    pub fn clear(&mut self) {
        unsafe { self.set_len(0); }
    }

    /// Extends the vector with a slice of bytes.
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

    // --- Helpers ---

    #[inline]
    unsafe fn set_len(&mut self, new_len: usize) {
        unsafe { ptr::write(self.ptr.as_ptr().add(Self::OFFSET_LEN) as *mut u32, new_len as u32) };
    }

    fn grow(&mut self, additional: usize) {
        let current_cap = self.capacity();
        let current_len = self.len();
        
        // Calculate new capacity (doubling strategy similar to std::Vec)
        let required_cap = current_len + Self::HEADER_SIZE + additional;
        let mut new_cap = current_cap * 2;
        if new_cap < required_cap {
            new_cap = required_cap;
        }
        
        // Ensure aligned to 16
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
            
            // Update the capacity field in the header at the *new* location
            ptr::write(new_ptr.add(Self::OFFSET_CAP) as *mut u32, new_cap as u32);
            
            self.ptr = NonNull::new_unchecked(new_ptr);
        }
    }
}

// --- Trait Implementations ---

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
