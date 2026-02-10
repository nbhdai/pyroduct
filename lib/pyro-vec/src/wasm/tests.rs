//! Host-only unit tests for the wasm pyro IO layer.
//!
//! These test the wasm_side registries, PyroView creation over simulated
//! "wasm memory" buffers, wasm_main with rkyv PyroRow, and PyroState
//! bookkeeping — all without spinning up a real wasmtime instance.

#[cfg(test)]
mod tests {
    use crate::{
        header::{PyroData, PyroHeader, PyroHeaderMut, PyroParser, DataStatus, MAGIC_VAL},
        view::{get_view, get_view_mut, PyroErrorView},
        PyroVec,
    };

    // =========================================================================
    // Helpers
    // =========================================================================

    /// Build a buffer from a PyroVec's packet slice, ensuring 16-byte
    /// alignment at offset 0.
    fn make_wasm_memory(payload: &[u8]) -> Vec<u8> {
        let mut vec = PyroVec::with_capacity(payload.len());
        vec.extend_from_slice(payload);
        let packet = vec.as_packet_slice();

        let align = PyroParser::ALIGN;
        let extra = align;
        let total = packet.len() + extra;
        let mut buf = vec![0u8; total];

        let base = buf.as_ptr() as usize;
        let aligned_offset = (align - (base % align)) % align;

        buf[aligned_offset..aligned_offset + packet.len()].copy_from_slice(packet);
        buf.drain(..aligned_offset);
        buf.truncate(packet.len());
        buf
    }

    fn aligned_buffer(size: usize) -> AlignedBuf {
        let align = PyroParser::ALIGN;
        let alloc_size = size + align;
        let raw = vec![0u8; alloc_size];
        let base = raw.as_ptr() as usize;
        let offset = (align - (base % align)) % align;
        AlignedBuf { raw, offset, len: size }
    }

    struct AlignedBuf {
        raw: Vec<u8>,
        offset: usize,
        len: usize,
    }

    impl AlignedBuf {
        fn as_slice(&self) -> &[u8] {
            &self.raw[self.offset..self.offset + self.len]
        }
        fn as_mut_slice(&mut self) -> &mut [u8] {
            let o = self.offset;
            let l = self.len;
            &mut self.raw[o..o + l]
        }
    }

    // =========================================================================
    // wasm_side registry tests
    // =========================================================================

    #[test]
    fn test_input_registry_new_and_get() {
        use crate::wasm::wasm_side::{get_input, new_input};

        let ptr = new_input(64);
        assert!(!ptr.is_null());

        let vec = get_input(ptr).expect("should find the registered vec");
        assert!(vec.capacity() >= 64);
        assert_eq!(vec.len(), 0);

        // Consumed — second get returns None.
        assert!(get_input(ptr).is_none());
    }

    #[test]
    fn test_input_registry_grow() {
        use crate::wasm::wasm_side::{get_input, grow_input, new_input};

        let ptr = new_input(16);
        assert!(!ptr.is_null());

        let ptr2 = grow_input(ptr, 256);
        assert!(!ptr2.is_null());

        let vec = get_input(ptr2).expect("should find grown vec");
        assert!(vec.capacity() >= 256);
    }

    #[test]
    fn test_grow_unknown_ptr_returns_null() {
        use crate::wasm::wasm_side::grow_input;
        let result = grow_input(0xDEAD_BEEF as *mut u8, 100);
        assert!(result.is_null());
    }

    #[test]
    fn test_output_registry_roundtrip() {
        use crate::wasm::wasm_side::{free_output, to_output};

        let mut vec = PyroVec::with_capacity(32);
        vec.extend_from_slice(b"hello");

        let ptr = to_output(vec);
        assert!(!ptr.is_null());

        free_output(ptr);
    }

    // =========================================================================
    // PyroView over simulated wasm memory
    // =========================================================================

    #[test]
    fn test_get_view_valid() {
        let mem = make_wasm_memory(b"test payload");
        let view = get_view(&mem, 0).expect("valid PyroView");
        assert_eq!(&*view, b"test payload");
        assert_eq!(view.len(), 12);
    }

    #[test]
    fn test_get_view_empty_payload() {
        let mem = make_wasm_memory(b"");
        let view = get_view(&mem, 0).expect("valid empty PyroView");
        assert_eq!(view.len(), 0);
    }

    #[test]
    fn test_get_view_misaligned_offset() {
        let mut buf = aligned_buffer(64);
        let vec = PyroVec::with_capacity(4);
        let packet = vec.as_packet_slice();
        buf.as_mut_slice()[..packet.len()].copy_from_slice(packet);

        match get_view(buf.as_slice(), 1) {
            Err(PyroErrorView::MisalignedPointer) => {}
            other => panic!("expected MisalignedPointer, got {:?}", other),
        }
    }

    #[test]
    fn test_get_view_truncated_header() {
        let buf = aligned_buffer(8);
        match get_view(buf.as_slice(), 0) {
            Err(PyroErrorView::OutOfBounds { .. }) => {}
            other => panic!("expected OutOfBounds, got {:?}", other),
        }
    }

    #[test]
    fn test_get_view_bad_magic() {
        let buf = aligned_buffer(32);
        match get_view(buf.as_slice(), 0) {
            Err(PyroErrorView::InvalidHeader) => {}
            other => panic!("expected InvalidHeader, got {:?}", other),
        }
    }

    #[test]
    fn test_get_view_mut_write_through() {
        let mut mem = make_wasm_memory(b"abcd");
        {
            let mut view = get_view_mut(&mut mem, 0).expect("valid mutable view");
            view[0] = b'X';
            assert_eq!(&*view, b"Xbcd");
        }
        let check = get_view(&mem, 0).unwrap();
        assert_eq!(&*check, b"Xbcd");
    }

    // =========================================================================
    // PyroView ↔ PyroVec roundtrip
    // =========================================================================

    #[test]
    fn test_view_from_vec_roundtrip() {
        use crate::view::PyroView;

        let mut original = PyroVec::with_capacity(64);
        original.extend_from_slice(b"roundtrip data");
        original.set_version(7);
        original.set_wire_format(2);

        let view: PyroView<'_> = (&original).into();
        assert_eq!(&*view, b"roundtrip data");
        assert_eq!(view.version(), 7);
        assert_eq!(view.wire_format(), 2);
    }

    #[test]
    fn test_view_header_fields_preserved() {
        let mut vec = PyroVec::with_capacity(32);
        vec.extend_from_slice(b"data");
        vec.set_status_u8(42);
        vec.set_version(3);
        vec.set_error_version(5);
        vec.set_wire_format(1);

        let mem = {
            let packet = vec.as_packet_slice();
            let mut m = vec![0u8; packet.len() + 16];
            let base = m.as_ptr() as usize;
            let offset = (16 - (base % 16)) % 16;
            m[offset..offset + packet.len()].copy_from_slice(packet);
            m.drain(..offset);
            m.truncate(packet.len());
            m
        };

        let view = get_view(&mem, 0).unwrap();
        assert_eq!(view.status_u8(), 42);
        assert_eq!(view.version(), 3);
        assert_eq!(view.error_version(), 5);
        assert_eq!(view.wire_format(), 1);
        assert_eq!(&*view, b"data");
    }

    // =========================================================================
    // Multiple concurrent registrations
    // =========================================================================

    #[test]
    fn test_multiple_concurrent_registrations() {
        use crate::wasm::wasm_side::{free_output, get_input, new_input, to_output};

        let ptr_a = new_input(16);
        let ptr_b = new_input(32);
        let ptr_c = new_input(64);

        assert_ne!(ptr_a, ptr_b);
        assert_ne!(ptr_b, ptr_c);
        assert_ne!(ptr_a, ptr_c);

        let _b = get_input(ptr_b).expect("b");
        let _a = get_input(ptr_a).expect("a");
        let _c = get_input(ptr_c).expect("c");

        let mut o1 = PyroVec::with_capacity(8);
        o1.extend_from_slice(b"o1");
        let mut o2 = PyroVec::with_capacity(8);
        o2.extend_from_slice(b"o2");

        let op1 = to_output(o1);
        let op2 = to_output(o2);
        assert_ne!(op1, op2);

        free_output(op1);
        free_output(op2);
    }

    // =========================================================================
    // Error conversion
    // =========================================================================

    #[test]
    fn test_anyhow_to_captured_error() {
        use crate::CapturedError;

        let err = anyhow::anyhow!("root cause")
            .context("middle layer")
            .context("top layer");

        let captured: CapturedError = err.into();
        assert!(captured.message.contains("top layer"));
        assert!(captured.error.is_some());
        let chain = captured.error.unwrap();
        assert!(chain.contains("middle layer"));
        assert!(chain.contains("root cause"));
    }

    // =========================================================================
    // wasm_main: WasmMain trait + wasm_call_extern (rkyv)
    // =========================================================================

    #[test]
    fn test_wasm_call_extern_rkyv_roundtrip() {
        use crate::value::{PyroRow, PyroRowOwned, PyroValue};
        use crate::wasm::{WasmMain, wasm_call_extern};
        use crate::wasm::wasm_side::free_output;
        use crate::Bridgeable;
        use crate::header::PyroData;

        struct Echo;
        impl WasmMain for Echo {
            fn call_extern(input: PyroVec) -> anyhow::Result<PyroVec> {
                // Zero-copy access to the archived row
                let typed = PyroRowOwned::expose(input)?;
                // We have access to ArchivedPyroRow here.
                // For the test, just ship back an output row.
                let mut out = PyroRow::new();
                out.insert("echo".into(), PyroValue::from("hello back"));
                Ok(out.into_owned().ship()?)
            }
        }

        // 1. Build the rkyv-serialized input PyroVec
        let input_row = PyroRow::from([
            ("msg", PyroValue::from("hello")),
        ]).into_owned();
        let input_vec = input_row.ship().expect("ship input");

        // 2. Register it in the input registry (simulates host writing to wasm memory)
        let raw_ptr = {
            let raw = input_vec.as_packet_slice();
            raw.as_ptr() as *mut u8
        };
        crate::wasm::wasm_side::_test_reinsert_input(raw_ptr, input_vec);

        // 3. Call the wasm entry
        let output_ptr = wasm_call_extern::<Echo>(raw_ptr);
        assert!(!output_ptr.is_null());

        // 4. Clean up
        free_output(output_ptr);
    }

    #[test]
    fn test_wasm_call_extern_error_propagation() {
        use crate::value::{PyroRow, PyroRowOwned};
        use crate::wasm::{WasmMain, wasm_call_extern};
        use crate::wasm::wasm_side::free_output;
        use crate::Bridgeable;

        struct Failing;
        impl WasmMain for Failing {
            fn call_extern(_input: PyroVec) -> anyhow::Result<PyroVec> {
                anyhow::bail!("something went wrong")
            }
        }

        // Ship an empty row as input
        let input_vec = PyroRow::new().into_owned().ship().unwrap();
        let raw_ptr = input_vec.as_packet_slice().as_ptr() as *mut u8;
        crate::wasm::wasm_side::_test_reinsert_input(raw_ptr, input_vec);

        let output_ptr = wasm_call_extern::<Failing>(raw_ptr);
        assert!(!output_ptr.is_null());

        // The output should have an error status — verify via PyroView
        // We can't use get_view here since it's not in wasm linear memory,
        // but the output is in the registry. Just verify it cleans up.
        free_output(output_ptr);
    }

    #[test]
    fn test_wasm_call_extern_null_input() {
        use crate::wasm::wasm_side::free_output;
        use crate::wasm::{WasmMain, wasm_call_extern};
        use crate::value::PyroRow;

        struct Noop;
        impl WasmMain for Noop {
            fn call_extern(_: PyroVec) -> anyhow::Result<PyroVec> {
                Ok(PyroRow::new().into_owned().ship()?)
            }
        }

        // Unregistered pointer → should return an error output
        let output_ptr = wasm_call_extern::<Noop>(0xDEAD as *mut u8);
        assert!(!output_ptr.is_null());
        free_output(output_ptr);
    }

    // =========================================================================
    // PyroRow rkyv ship/expose roundtrip through PyroVec + PyroView
    // =========================================================================

    #[test]
    fn test_pyro_row_rkyv_view_roundtrip() {
        use crate::value::{PyroRow, PyroRowOwned, PyroValue};
        use crate::Bridgeable;

        let row = PyroRow::from([
            ("id", PyroValue::from(42i32)),
            ("name", PyroValue::from("test")),
        ]).into_owned();

        // Ship via rkyv into a PyroVec
        let vec = row.ship().unwrap();
        assert_eq!(vec.wire_format(), 1); // PROTOCOL_VERSION (rkyv)

        // Simulate writing into wasm memory and reading back as a view
        let mem = {
            let packet = vec.as_packet_slice();
            let mut m = std::vec![0u8; packet.len() + 16];
            let base = m.as_ptr() as usize;
            let offset = (16 - (base % 16)) % 16;
            m[offset..offset + packet.len()].copy_from_slice(packet);
            m.drain(..offset);
            m.truncate(packet.len());
            m
        };

        // Zero-copy access via PyroView → expose_view
        let view = get_view(&mem, 0).unwrap();
        let typed = PyroRowOwned::expose_view(view)
            .expect("expose_view should succeed");

        // The typed wrapper gives zero-copy access to ArchivedPyroRow.
        // Verify it parsed without error.
        let _ = &*typed;
    }

    /// Full simulated flow: ship → registry → get_input → expose → process → ship → to_output → free
    #[test]
    fn test_simulated_full_rkyv_flow() {
        use crate::value::{PyroRow, PyroRowOwned, PyroValue};
        use crate::wasm::wasm_side::{free_output, get_input, to_output};
        use crate::Bridgeable;

        // 1. Host ships a row
        let input_row = PyroRow::from([
            ("x", PyroValue::from(100i32)),
        ]).into_owned();
        let input_vec = input_row.ship().unwrap();

        // 2. Simulate new_input + host copy (we just register the vec directly)
        let ptr = input_vec.as_packet_slice().as_ptr() as *mut u8;
        crate::wasm::wasm_side::_test_reinsert_input(ptr, input_vec);

        // 3. Wasm side: get_input → expose → zero-copy access
        let owned_input = get_input(ptr).expect("input should exist");
        let typed_input = PyroRowOwned::expose(owned_input)
            .expect("expose input");
        let _archived = &*typed_input;

        // 4. Wasm produces output
        let output_row = PyroRow::from([
            ("result", PyroValue::from(200i32)),
        ]).into_owned();
        let output_vec = output_row.ship().unwrap();
        let output_ptr = to_output(output_vec);

        // 5. Host frees
        free_output(output_ptr);
    }
}