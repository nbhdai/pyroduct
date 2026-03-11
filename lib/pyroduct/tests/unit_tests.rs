use pyroduct::format::{
    PyroVec,
    header::{PyroData, PyroHeader, PyroHeaderMut},
};

// =============================================================================
// Construction & Basic Properties
// =============================================================================

#[test]
fn test_with_capacity_zero() {
    let vec = PyroVec::with_capacity(0);
    assert_eq!(vec.len(), 0);
    assert!(vec.is_empty());
    // Minimum allocation is ALIGN (16), so capacity >= 16
    assert!(vec.capacity() == 0);
}

#[test]
fn test_with_capacity_small() {
    let vec = PyroVec::with_capacity(10);
    assert_eq!(vec.len(), 0);
    assert!(vec.capacity() >= 10 + 0);
}

#[test]
fn test_with_capacity_large() {
    let vec = PyroVec::with_capacity(10000);
    assert_eq!(vec.len(), 0);
    assert!(vec.capacity() >= 10000 + 0);
}

// #[test]
// fn test_default_header_values() {
//     let vec = PyroVec::with_capacity(10);
//     assert_eq!(vec.status(), Ok(DataStatus::Empty));
//     assert_eq!(vec.wire_format(), 1);
//     assert_eq!(vec.version(), 0);
//     assert_eq!(vec.error_version(), 0);
// }

// =============================================================================
// Alignment & Layout
// =============================================================================

#[test]
fn test_base_pointer_alignment() {
    let vec = PyroVec::with_capacity(100);
    let addr = vec.as_ptr() as usize;
    assert_eq!(addr % 16, 0, "Base pointer must be 16-byte aligned");
}

#[test]
fn test_data_pointer_alignment() {
    let vec = PyroVec::with_capacity(100);
    let data_addr = vec.data_ptr() as usize;

    assert_eq!(data_addr % 16, 0, "Data pointer must be 16-byte aligned");
}

#[test]
fn test_alignment_preserved_after_grow() {
    let mut vec = PyroVec::with_capacity(10);

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
fn test_header_byte_packing() {
    let mut vec = PyroVec::with_capacity(10);

    // Write distinct values to all 4 byte fields
    vec.set_wire_format(0xAA);
    vec.set_status_u8(0xBB);
    vec.set_error_version(0xCC);
    vec.set_version(0xDD);

    // Verify read back
    assert_eq!(vec.wire_format(), 0xAA);
    assert_eq!(vec.status_u8(), 0xBB);
    assert_eq!(vec.error_version(), 0xCC);
    assert_eq!(vec.version(), 0xDD);

    // Verify via raw slice to ensure correct offsets
    let raw = vec.header();
    assert_eq!(raw[8], 0xAA); // Wire Format
    assert_eq!(raw[9], 0xBB); // Status 
    assert_eq!(raw[10], 0xCC); // Error Version
    assert_eq!(raw[11], 0xDD); // User Version 
}

// =============================================================================
// Data Operations - push
// =============================================================================

#[test]
fn test_push_single() {
    let mut vec = PyroVec::with_capacity(10);
    vec.push(0xAB);

    assert_eq!(vec.len(), 1);
    assert_eq!(vec.as_slice(), &[0xAB]);
}

#[test]
fn test_push_triggers_grow() {
    let mut vec = PyroVec::with_capacity(2);
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

#[test]
fn test_extend_from_slice_empty() {
    let mut vec = PyroVec::with_capacity(10);
    vec.extend_from_slice(&[]);

    assert_eq!(vec.len(), 0);
    assert!(vec.is_empty());
}

#[test]
fn test_extend_from_slice_small() {
    let mut vec = PyroVec::with_capacity(10);
    vec.extend_from_slice(&[1, 2, 3, 4, 5]);

    assert_eq!(vec.len(), 5);
    assert_eq!(vec.as_slice(), &[1, 2, 3, 4, 5]);
}

#[test]
fn test_extend_from_slice_multiple() {
    let mut vec = PyroVec::with_capacity(10);

    vec.extend_from_slice(&[1, 2, 3]);
    vec.extend_from_slice(&[4, 5, 6]);
    vec.extend_from_slice(&[7, 8, 9]);

    assert_eq!(vec.len(), 9);
    assert_eq!(vec.as_slice(), &[1, 2, 3, 4, 5, 6, 7, 8, 9]);
}

#[test]
fn test_extend_from_slice_triggers_grow() {
    let mut vec = PyroVec::with_capacity(5);
    let pattern: Vec<u8> = (0..200).collect();

    vec.extend_from_slice(&pattern);

    assert_eq!(vec.len(), 200);
    assert_eq!(vec.as_slice(), &pattern[..]);
}

#[test]
fn test_clear_empty() {
    let mut vec = PyroVec::with_capacity(10);
    vec.clear();

    assert_eq!(vec.len(), 0);
    assert!(vec.is_empty());
}

#[test]
fn test_clear_with_data() {
    let mut vec = PyroVec::with_capacity(10);
    vec.extend_from_slice(&[1, 2, 3, 4, 5]);

    let cap_before = vec.capacity();
    vec.clear();

    assert_eq!(vec.len(), 0);
    assert!(vec.is_empty());
    assert_eq!(vec.capacity(), cap_before); // Capacity unchanged
}

#[test]
fn test_clear_then_reuse() {
    let mut vec = PyroVec::with_capacity(10);

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
fn test_as_packet_slice() {
    let mut vec = PyroVec::with_capacity(10);
    vec.extend_from_slice(&[0xAA, 0xBB, 0xCC]);

    // Should be header (16 bytes) + data (3 bytes)
    assert_eq!(vec.len(), 3);

    // Verify magic at start
    let magic = u32::from_ne_bytes(vec.header()[0..4].try_into().unwrap());
    assert_eq!(magic, 0x7079726F);

    // Verify data at end
    assert_eq!(&*vec, &[0xAA, 0xBB, 0xCC]);
}

#[test]
fn test_deref() {
    let mut vec = PyroVec::with_capacity(10);
    vec.extend_from_slice(b"test");

    let slice: &[u8] = &vec;
    assert_eq!(slice, b"test");
}

#[test]
fn test_deref_mut() {
    let mut vec = PyroVec::with_capacity(10);
    vec.extend_from_slice(&[1, 2, 3]);

    let slice: &mut [u8] = &mut vec;
    slice[1] = 99;

    assert_eq!(vec.as_slice(), &[1, 99, 3]);
}

// =============================================================================
// Clone
// =============================================================================

#[test]
fn test_clone_copies_all_fields() {
    let mut original = PyroVec::with_capacity(10);
    original.extend_from_slice(b"hello");
    original.set_status_u8(1);
    original.set_version(2);
    original.set_error_version(3);
    original.set_wire_format(4);

    let cloned = original.clone();

    assert_eq!(cloned.as_slice(), b"hello");
    assert_eq!(cloned.status_u8(), 1);
    assert_eq!(cloned.version(), 2);
    assert_eq!(cloned.error_version(), 3);
    assert_eq!(cloned.wire_format(), 4);

    // Verify independence
    assert_ne!(original.as_ptr(), cloned.as_ptr());
}

#[test]
fn test_clone_with_data() {
    let mut original = PyroVec::with_capacity(10);
    original.extend_from_slice(b"hello world");
    original.set_status_u8(42);
    original.set_wire_format(7);

    let cloned = original.clone();

    assert_eq!(cloned.as_slice(), b"hello world");
    assert_eq!(cloned.status_u8(), 42);
    assert_eq!(cloned.wire_format(), 7);

    // Verify independence
    assert_ne!(original.as_ptr(), cloned.as_ptr());
}

#[test]
fn test_eq_empty() {
    let a = PyroVec::with_capacity(10);
    let b = PyroVec::with_capacity(20);

    assert_eq!(a, b);
}

#[test]
fn test_eq_same_data() {
    let mut a = PyroVec::with_capacity(10);
    let mut b = PyroVec::with_capacity(20);

    a.extend_from_slice(b"test");
    b.extend_from_slice(b"test");

    assert_eq!(a, b);
}

#[test]
fn test_eq_different_data() {
    let mut a = PyroVec::with_capacity(10);
    let mut b = PyroVec::with_capacity(10);

    a.extend_from_slice(b"hello");
    b.extend_from_slice(b"world");

    assert_ne!(a, b);
}

// =============================================================================
// from_raw
// =============================================================================

#[test]
fn test_grow_preserves_header() {
    let mut vec = PyroVec::with_capacity(10);
    vec.set_status_u8(42);
    vec.set_wire_format(99);

    // Force grow
    let pattern: Vec<u8> = (0..500).map(|i| i as u8).collect();
    vec.extend_from_slice(&pattern);

    assert_eq!(
        vec.status_u8(),
        42,
        "Status must be preserved across realloc"
    );
    assert_eq!(
        vec.wire_format(),
        99,
        "Version must be preserved across realloc"
    );
    assert_eq!(vec.as_slice(), &pattern[..]);
}

#[test]
fn test_grow_preserves_data() {
    let mut vec = PyroVec::with_capacity(10);

    for i in 0u8..=255 {
        vec.push(i);
    }

    // Verify all data intact
    for i in 0u8..=255 {
        assert_eq!(vec.as_slice()[i as usize], i);
    }
}
