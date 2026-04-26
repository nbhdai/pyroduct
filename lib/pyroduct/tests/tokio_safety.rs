use pyroduct::format::{
    PyroVec,
    header::{DataStatus, PyroHeader, PyroHeaderMut},
    tokio::{read_from_stream, write_to_stream},
};
use std::io::Cursor;

#[tokio::test]
async fn test_stream_roundtrip() {
    let mut original = PyroVec::with_capacity(32);
    original.extend_from_slice(b"Async Data Packet");
    original.set_status(DataStatus::RemoteUtf8);
    original.set_fn_id(2);

    let mut stream_buffer = Vec::new();

    write_to_stream(&mut stream_buffer, &original, None)
        .await
        .expect("Write failed");

    let mut cursor = Cursor::new(stream_buffer);
    let recovered = read_from_stream(&mut cursor, None)
        .await
        .expect("Read failed");

    assert_eq!(recovered.as_slice(), b"Async Data Packet");
    assert_eq!(recovered.len(), 17);
    assert_eq!(
        recovered.status(),
        Ok(DataStatus::RemoteUtf8),
        "Status must survive roundtrip"
    );
    assert_eq!(recovered.fn_id(), 2, "Function ID must survive roundtrip");
}

#[tokio::test]
async fn test_stream_preserves_header_fields() {
    let mut original = PyroVec::with_capacity(8);
    original.extend_from_slice(b"data");
    original.set_status(DataStatus::LocalSerialization);
    original.set_fn_id(0x12);

    let mut stream_buffer = Vec::new();
    write_to_stream(&mut stream_buffer, &original, None)
        .await
        .unwrap();

    let mut cursor = Cursor::new(stream_buffer);
    let recovered = read_from_stream(&mut cursor, None).await.unwrap();

    assert_eq!(recovered.status(), Ok(DataStatus::LocalSerialization));
    assert_eq!(recovered.fn_id(), 0x12);
}

#[tokio::test]
async fn test_read_rejects_bad_magic() {
    let mut bad_packet = Vec::new();
    // Header
    bad_packet.extend_from_slice(&0xDEADBEEFu32.to_ne_bytes()); // BAD Magic
    bad_packet.extend_from_slice(&10u32.to_ne_bytes()); // Len
    bad_packet.extend_from_slice(&[0u8; 8]); // Rest of header (8 bytes)
    // Body
    bad_packet.extend_from_slice(&[0u8; 10]);

    let mut cursor = Cursor::new(bad_packet);
    let result = read_from_stream(&mut cursor, None).await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::InvalidData);
}

#[tokio::test]
async fn test_read_detects_header_eof() {
    // Header is 16 bytes. We only provide 10.
    let partial_header = vec![0u8; 10];
    let mut cursor = Cursor::new(partial_header);

    let result = read_from_stream(&mut cursor, None).await;

    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().kind(),
        std::io::ErrorKind::UnexpectedEof
    );
}

#[tokio::test]
async fn test_read_detects_body_eof() {
    let body_len: u32 = 10;
    let mut packet = Vec::new();

    // Header
    packet.extend_from_slice(&0x7079726Fu32.to_ne_bytes()); // Valid Magic
    packet.extend_from_slice(&body_len.to_ne_bytes()); // Len = 10
    packet.extend_from_slice(&[0u8; 8]); // Rest of header (8 bytes)

    // Body (Only 5 bytes, but we promised 10)
    packet.extend_from_slice(b"12345");

    let mut cursor = Cursor::new(packet);
    let result = read_from_stream(&mut cursor, None).await;

    assert!(result.is_err());
    // Should fail because it couldn't fill the buffer
    assert_eq!(
        result.unwrap_err().kind(),
        std::io::ErrorKind::UnexpectedEof
    );
}
