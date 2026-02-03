use std::alloc::{Layout, alloc, dealloc};
use std::ptr;

// REPLACE THIS with your actual crate name
use len_aligned_vec::LenAlignedVec;

#[test]
fn test_alignment_and_layout_contract() {
    let vec = LenAlignedVec::with_capacity(100);
    
    // 1. Base Pointer Alignment
    let raw_addr = vec.as_ptr() as usize;
    assert_eq!(raw_addr % 16, 0, "Base pointer must be 16-byte aligned");

    // 2. Data Pointer Alignment
    // The data payload starts exactly 16 bytes after the base pointer
    let data_addr = vec.data_ptr() as usize;
    assert_eq!(data_addr % 16, 0, "Data pointer must be 16-byte aligned");
    assert_eq!(data_addr - raw_addr, 16, "Header size must be exactly 16 bytes");
}

#[test]
fn test_raw_pointer_reconstruction() {
    // Setup initial state
    let mut original = LenAlignedVec::with_capacity(50);
    original.extend_from_slice(&[0xAA, 0xBB, 0xCC]);
    original.set_status(7);

    let raw_ptr = original.as_ptr();

    // Reconstruct. Note: This creates a SECOND owner of the same memory.
    // We must be careful not to double-free.
    let reconstructed = unsafe { 
        LenAlignedVec::from_raw(raw_ptr).expect("Should reconstruct from valid ptr") 
    };
    
    assert_eq!(reconstructed.len(), 3);
    assert_eq!(reconstructed.status(), 7);
    assert_eq!(reconstructed.as_slice(), &[0xAA, 0xBB, 0xCC]);

    // Safety: Prevent double free. `original` and `reconstructed` point to the same buffer.
    // We forget `original` so only `reconstructed` drops the memory.
    std::mem::forget(original);
}

#[test]
fn test_guard_invalid_magic() {
    // Manually alloc memory that looks like a LenAlignedVec but has bad magic
    let layout = Layout::from_size_align(32, 16).unwrap();
    let ptr = unsafe { alloc(layout) };
    
    unsafe {
        // Write BAD magic (0xDEADBEEF instead of 0x7079726F) at offset 0
        ptr::write(ptr as *mut u32, 0xDEADBEEF); 
    }

    let result = unsafe { LenAlignedVec::from_raw(ptr) };
    assert!(result.is_err());
    assert_eq!(result.err(), Some("Invalid magic header"));

    // Clean up manually since LenAlignedVec didn't take ownership
    unsafe { dealloc(ptr, layout); }
}

#[test]
fn test_guard_misalignment() {
    let layout = Layout::from_size_align(64, 16).unwrap();
    let ptr = unsafe { alloc(layout) };
    
    // Create a pointer offset by 1 byte (misaligned)
    let bad_ptr = unsafe { ptr.add(1) };

    let result = unsafe { LenAlignedVec::from_raw(bad_ptr) };
    assert!(result.is_err());
    assert_eq!(result.err(), Some("Pointer is not 16-byte aligned"));

    unsafe { dealloc(ptr, layout); }
}

#[test]
fn test_grow_preserves_header_and_data() {
    let mut vec = LenAlignedVec::with_capacity(10);
    vec.set_status(42);
    let pattern: Vec<u8> = (0..50).collect();
    
    // This will force multiple reallocations
    vec.extend_from_slice(&pattern); 
    
    assert_eq!(vec.len(), 50);
    assert_eq!(vec.status(), 42, "Status must be preserved across realloc");
    assert_eq!(vec.as_slice(), &pattern[..]);
    
    // Verify alignment is still correct after realloc
    let addr = vec.as_ptr() as usize;
    assert_eq!(addr % 16, 0);
}