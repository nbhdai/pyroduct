//! Host-only unit tests for the wasm pyro IO layer.
//!
//! These test the wasm_side registries, PyroView creation over simulated
//! "wasm memory" buffers, wasm_main with rkyv PyroRow, and PyroState
//! bookkeeping — all without spinning up a real wasmtime instance.

use crate::{
    ParseError, PyroError, PyroVec,
    header::{PyroData, PyroHeader, PyroHeaderMut, PyroParser},
    view::{get_view, get_view_mut},
    wasm::{free_output, get_input, grow_input, new_input, to_output},
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
    AlignedBuf {
        raw,
        offset,
        len: size,
    }
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
    let ptr = new_input(16);
    assert!(!ptr.is_null());

    let ptr2 = grow_input(ptr, 256);
    assert!(!ptr2.is_null());

    let vec = get_input(ptr2).expect("should find grown vec");
    assert!(vec.capacity() >= 256);
}

#[test]
fn test_grow_unknown_ptr_returns_null() {
    let result = grow_input(0xDEAD_BEEF as *mut u8, 100);
    assert!(result.is_null());
}

#[test]
fn test_output_registry_roundtrip() {
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
        Err(PyroError::Header(ParseError::MisalignedPointer)) => {}
        other => panic!("expected MisalignedPointer, got {:?}", other),
    }
}

#[test]
fn test_get_view_truncated_header() {
    let buf = aligned_buffer(8);
    match get_view(buf.as_slice(), 0) {
        Err(PyroError::Header(ParseError::SliceTooSmall)) => {}
        other => panic!("expected OutOfBounds, got {:?}", other),
    }
}

#[test]
fn test_get_view_bad_magic() {
    let buf = aligned_buffer(32);
    match get_view(buf.as_slice(), 0) {
        Err(PyroError::Header(ParseError::InvalidMagic)) => {}
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
// PyroRow rkyv ship/expose roundtrip through PyroVec + PyroView
// =========================================================================

#[test]
fn test_pyro_row_rkyv_view_roundtrip() {
    use crate::Bridgeable;
    use crate::value::{PyroRow, PyroRowOwned, PyroValue};

    let row = PyroRow::from([
        ("id", PyroValue::from(42i32)),
        ("name", PyroValue::from("test")),
    ])
    .into_owned();

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
    let typed = PyroRowOwned::expose_view(view).expect("expose_view should succeed");

    // The typed wrapper gives zero-copy access to ArchivedPyroRow.
    // Verify it parsed without error.
    let _ = &*typed;
}
