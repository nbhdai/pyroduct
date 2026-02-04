use bridge_vec::{BridgeVec, captured::{BridgeError, ErrorKind, ErrorOrigin}};

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
        assert_eq!(vec.version(), 0);
        assert_eq!(vec.error_version(), 0);
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
        assert_eq!(
            data_addr - base_addr,
            16,
            "Header size must be exactly 16 bytes"
        );
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
    fn test_header_byte_packing() {
        let mut vec = BridgeVec::with_capacity(10);

        // Write distinct values to all 4 byte fields
        vec.set_wire_format(0xAA);
        vec.set_version(0xBB);
        vec.set_error_version(0xCC);
        vec.set_status_u8(0xDD);

        // Verify read back
        assert_eq!(vec.wire_format(), 0xAA);
        assert_eq!(vec.version(), 0xBB);
        assert_eq!(vec.error_version(), 0xCC);
        assert_eq!(vec.status(), 0xDD);

        // Verify via raw slice to ensure correct offsets
        let raw = vec.as_packet_slice();
        assert_eq!(raw[12], 0xAA); // Wire Format
        assert_eq!(raw[13], 0xBB); // User Version
        assert_eq!(raw[14], 0xCC); // Error Version
        assert_eq!(raw[15], 0xDD); // Status
    }

    #[test]
    fn test_status_safety() {
        // In previous buggy versions, writing status as u16 would overwrite data
        let mut vec = BridgeVec::with_capacity(1);
        vec.push(0xFF); // Data at offset 16

        vec.set_status_u8(0xEE); // Write to offset 15

        assert_eq!(vec.status(), 0xEE);
        assert_eq!(
            vec.as_slice()[0],
            0xFF,
            "Setting status must not corrupt data"
        );
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
    fn test_clone_copies_all_fields() {
        let mut original = BridgeVec::with_capacity(10);
        original.extend_from_slice(b"hello");
        original.set_status_u8(1);
        original.set_version(2);
        original.set_error_version(3);
        original.set_wire_format(4);

        let cloned = original.clone();

        assert_eq!(cloned.as_slice(), b"hello");
        assert_eq!(cloned.status(), 1);
        assert_eq!(cloned.version(), 2);
        assert_eq!(cloned.error_version(), 3);
        assert_eq!(cloned.wire_format(), 4);

        // Verify independence
        assert_ne!(original.as_ptr(), cloned.as_ptr());
    }

    #[test]
    fn test_clone_with_data() {
        let mut original = BridgeVec::with_capacity(10);
        original.extend_from_slice(b"hello world");
        original.set_status_u8(42);
        original.set_wire_format(7);

        let cloned = original.clone();

        assert_eq!(cloned.as_slice(), b"hello world");
        assert_eq!(cloned.status(), 42);
        assert_eq!(cloned.wire_format(), 7);

        // Verify independence
        assert_ne!(original.as_ptr(), cloned.as_ptr());
    }

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

    // =============================================================================
    // from_raw
    // =============================================================================

    #[test]
    fn test_from_raw_null() {
        let result = unsafe { BridgeVec::from_raw(std::ptr::null()) };
        assert!(matches!(
            result,
            Err(BridgeError::Bridge {
                origin: ErrorOrigin::Local,
                kind: ErrorKind::NullPointer
            })
        ));
    }

    #[test]
    fn test_from_raw_misaligned() {
        let layout = Layout::from_size_align(64, 16).unwrap();
        let ptr = unsafe { alloc(layout) };

        let bad_ptr = unsafe { ptr.add(1) };
        let result = unsafe { BridgeVec::from_raw(bad_ptr) };

        assert!(matches!(
            result,
            Err(BridgeError::Bridge {
                origin: ErrorOrigin::Local,
                kind: ErrorKind::MisalignedPointer
            })
        ));

        unsafe { dealloc(ptr, layout) };
    }

    #[test]
    fn test_from_raw_bad_magic() {
        let layout = Layout::from_size_align(32, 16).unwrap();
        let ptr = unsafe { alloc(layout) };

        unsafe { ptr::write(ptr as *mut u32, 0xDEADBEEF) };

        let result = unsafe { BridgeVec::from_raw(ptr) };

        assert!(matches!(
            result,
            Err(BridgeError::Bridge {
                origin: ErrorOrigin::Local,
                kind: ErrorKind::InvalidHeader
            })
        ));

        unsafe { dealloc(ptr, layout) };
    }

    #[test]
    fn test_from_raw_valid() {
        let mut original = BridgeVec::with_capacity(50);
        original.extend_from_slice(&[0xAA, 0xBB, 0xCC]);
        original.set_status_u8(7);

        let raw_ptr = original.into_raw();

        let reconstructed =
            unsafe { BridgeVec::from_raw(raw_ptr).expect("Should reconstruct from valid ptr") };

        assert_eq!(reconstructed.len(), 3);
        assert_eq!(reconstructed.status(), 7);
        assert_eq!(reconstructed.as_slice(), &[0xAA, 0xBB, 0xCC]);
    }

    #[test]
    fn test_into_raw_ownership_transfer() {
        let mut vec = BridgeVec::with_capacity(10);
        vec.extend_from_slice(b"test");
        vec.set_status_u8(42);

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
        vec.set_status_u8(0x12);
        vec.set_wire_format(0x56);

        let ptr = vec.into_raw();
        let recovered = unsafe { BridgeVec::from_raw(ptr).unwrap() };

        assert_eq!(recovered.as_slice(), b"roundtrip test data");
        assert_eq!(recovered.status(), 0x12);
        assert_eq!(recovered.wire_format(), 0x56);
    }

    #[test]
    fn test_borrow_from_raw_null() {
        let result = unsafe { BridgeVec::borrow_raw(std::ptr::null()) };
        assert!(matches!(
            result,
            Err(BridgeError::Bridge {
                origin: ErrorOrigin::Local,
                kind: ErrorKind::NullPointer
            })
        ));
    }

    #[test]
    fn test_borrow_from_raw_misaligned() {
        let layout = Layout::from_size_align(64, 16).unwrap();
        let ptr = unsafe { alloc(layout) };

        let bad_ptr = unsafe { ptr.add(1) };
        let result = unsafe { BridgeVec::borrow_raw(bad_ptr) };

        assert!(matches!(
            result,
            Err(BridgeError::Bridge {
                origin: ErrorOrigin::Local,
                kind: ErrorKind::MisalignedPointer
            })
        ));

        unsafe { dealloc(ptr, layout) };
    }

    #[test]
    fn test_borrow_from_raw_bad_magic() {
        let layout = Layout::from_size_align(32, 16).unwrap();
        let ptr = unsafe { alloc(layout) };

        unsafe { ptr::write(ptr as *mut u32, 0xDEADBEEF) };

        let result = unsafe { BridgeVec::borrow_raw(ptr) };

        assert!(matches!(
            result,
            Err(BridgeError::Bridge {
                origin: ErrorOrigin::Local,
                kind: ErrorKind::InvalidHeader
            })
        ));

        unsafe { dealloc(ptr, layout) };
    }

    #[test]
    fn test_borrow_raw_non_owning() {
        let mut original = BridgeVec::with_capacity(50);
        original.extend_from_slice(&[0xAA, 0xBB, 0xCC]);
        original.set_status_u8(7);
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
            let _borrowed = unsafe { BridgeVec::borrow_raw(original.as_ptr()).unwrap() };
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
        vec.extend_from_slice(b"ref");
        vec.set_status_u8(10);
        vec.set_error_version(20);

        let borrowed = unsafe { BridgeVec::borrow_raw(vec.as_ptr()).unwrap() };

        assert_eq!(borrowed.status(), 10);
        assert_eq!(borrowed.error_version(), 20);
        assert_eq!(borrowed.as_slice(), b"ref");
    }

    #[test]
    fn test_grow_preserves_header() {
        let mut vec = BridgeVec::with_capacity(10);
        vec.set_status_u8(42);
        vec.set_wire_format(99);

        // Force grow
        let pattern: Vec<u8> = (0..500).map(|i| i as u8).collect();
        vec.extend_from_slice(&pattern);

        assert_eq!(vec.status(), 42, "Status must be preserved across realloc");
        assert_eq!(
            vec.wire_format(),
            99,
            "Version must be preserved across realloc"
        );
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
        vec.set_status_u8(100);
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
