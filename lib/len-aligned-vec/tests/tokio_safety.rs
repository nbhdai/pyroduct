use len_aligned_vec::{LenAlignedVec, tokio::{read_from_stream, write_to_stream}};
use std::io::Cursor;

#[tokio::test]
async fn test_stream_roundtrip() {
    let mut original = LenAlignedVec::with_capacity(32);
    original.extend_from_slice(b"Async Data Packet");
    // Note: implementation of write_to_stream currently writes 0 for status/version padding
    // so we don't expect status to survive a network roundtrip in the current code,
    // only the payload.

    let mut stream_buffer = Vec::new();
    
    // 1. Write to stream
    write_to_stream(&mut stream_buffer, &original).await.expect("Write failed");

    // 2. Read from stream
    let mut cursor = Cursor::new(stream_buffer);
    let recovered = read_from_stream(&mut cursor).await.expect("Read failed");

    assert_eq!(recovered.as_slice(), b"Async Data Packet");
    assert_eq!(recovered.len(), 17);
}

#[tokio::test]
async fn test_read_rejects_bad_magic() {
    let mut bad_packet = Vec::new();
    // Header
    bad_packet.extend_from_slice(&0xDEADBEEFu32.to_ne_bytes()); // BAD Magic
    bad_packet.extend_from_slice(&10u32.to_ne_bytes()); // Len
    bad_packet.extend_from_slice(&20u32.to_ne_bytes()); // Cap
    bad_packet.extend_from_slice(&0u32.to_ne_bytes()); // Padding
    // Body
    bad_packet.extend_from_slice(&[0u8; 10]);

    let mut cursor = Cursor::new(bad_packet);
    let result = read_from_stream(&mut cursor).await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::InvalidData);
}

#[tokio::test]
async fn test_read_detects_header_eof() {
    // Header is 16 bytes. We only provide 10.
    let partial_header = vec![0u8; 10]; 
    let mut cursor = Cursor::new(partial_header);
    
    let result = read_from_stream(&mut cursor).await;
    
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::UnexpectedEof);
}

#[tokio::test]
async fn test_read_detects_body_eof() {
    let body_len: u32 = 10;
    let mut packet = Vec::new();
    
    // Header
    packet.extend_from_slice(&0x7079726Fu32.to_ne_bytes()); // Valid Magic
    packet.extend_from_slice(&body_len.to_ne_bytes()); // Len = 10
    packet.extend_from_slice(&20u32.to_ne_bytes()); // Cap
    packet.extend_from_slice(&0u32.to_ne_bytes()); // Padding
    
    // Body (Only 5 bytes, but we promised 10)
    packet.extend_from_slice(b"12345");

    let mut cursor = Cursor::new(packet);
    let result = read_from_stream(&mut cursor).await;

    assert!(result.is_err());
    // Should fail because it couldn't fill the buffer
    assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::UnexpectedEof);
}