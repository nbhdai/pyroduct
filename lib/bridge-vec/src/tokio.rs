//! Tokio integration

use tokio::io::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::io::{Error, ErrorKind};

use crate::BridgeVec;

// --- AsyncWrite Implementation ---
// Allows BridgeVec to be used as a buffer for tokio::io::copy or other async writers.
impl AsyncWrite for BridgeVec {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        self.extend_from_slice(buf);
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }
}

// --- Framing Helpers ---

/// Reads a `BridgeVec` from an async stream (TCP/Unix).
/// 
/// This performs the framing logic:
/// 1. Reads the 16-byte header.
/// 2. Validates the Magic number.
/// 3. Reads the length, capacity, versions, and status.
/// 4. Allocates the vector.
/// 5. Reads the exact payload into the vector.
pub async fn read_from_stream<R>(src: &mut R) -> io::Result<BridgeVec> 
where 
    R: AsyncRead + Unpin
{
    let mut header_buf = [0u8; 16];
    
    // 1. Read the full 16-byte header
    src.read_exact(&mut header_buf).await?;

    // 2. Parse fields (Little Endian for multi-byte integers)
    // 0x00 - 0x03: Magic
    let magic = u32::from_le_bytes(header_buf[0..4].try_into().unwrap());
    
    // 0x04 - 0x07: Length
    let len = u32::from_le_bytes(header_buf[4..8].try_into().unwrap()) as usize;
    
    // 0x08 - 0x0B: Capacity
    let cap = u32::from_le_bytes(header_buf[8..12].try_into().unwrap()) as usize;
    
    // 0x0C: Wire Format
    let wire_format = header_buf[12];
    
    // 0x0D: User Version
    let version = header_buf[13];
    
    // 0x0E: Error Version
    let error_version = header_buf[14];
    
    // 0x0F: Status
    let status = header_buf[15];

    if magic != crate::MAGIC_VAL {
        return Err(Error::new(ErrorKind::InvalidData, "Invalid BridgeVec magic header"));
    }

    // 3. Allocate and set metadata
    let mut vec = BridgeVec::with_capacity(cap);
    vec.set_wire_format(wire_format);
    vec.set_version(version);
    vec.set_error_version(error_version);
    vec.set_status(status);

    // 4. Read Payload
    if len > 0 {
        let mut reader = src.take(len as u64);
        let bytes_read = io::copy(&mut reader, &mut vec).await?;
        
        if bytes_read != len as u64 {
            return Err(Error::new(ErrorKind::UnexpectedEof, "Stream ended before full payload was received"));
        }
    }

    debug_assert_eq!(len, vec.len());

    Ok(vec)
}

/// Helper to write a BridgeVec to an async stream.
/// This writes the header (with version/status) followed by the data payload.
pub async fn write_to_stream<W>(dest: &mut W, vec: &BridgeVec) -> io::Result<()>
where
    W: AsyncWrite + Unpin
{
    // Use native-endian writes to match the read_from_stream logic.
    // Note: Use write_all for single bytes to avoid endian confusion, though u8 is safe.

    // 0x00: Magic
    dest.write_u32_le(crate::MAGIC_VAL).await?;  
    
    // 0x04: Length
    dest.write_u32_le(vec.len() as u32).await?;          
    
    // 0x08: Capacity
    dest.write_u32_le(vec.capacity() as u32).await?;     
    
    // 0x0C: Wire Format
    dest.write_u8(vec.wire_format()).await?;             
    
    // 0x0D: User Version
    dest.write_u8(vec.version()).await?;             

    // 0x0E: Error Version
    dest.write_u8(vec.error_version()).await?;

    // 0x0F: Status
    dest.write_u8(vec.status()).await?;              

    // Payload
    dest.write_all(vec.as_slice()).await?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BridgeVec;
    use std::io::Cursor;

    #[tokio::test]
    async fn test_streaming_endian_consistency() {
        // Create a vec with specific metadata to test byte-order
        let mut original = BridgeVec::with_capacity(16);
        original.extend_from_slice(b"endian-test-data");
        
        // Use distinct values for all byte fields
        original.set_wire_format(0xAA);
        original.set_version(0xBB);
        original.set_error_version(0xCC);
        original.set_status(0xDD);

        let mut stream = Vec::new();

        // Step 1: Write to stream
        write_to_stream(&mut stream, &original)
            .await
            .expect("Failed to write to stream");

        // Step 2: Manually inspect the first 4 bytes (Magic)
        // If this fails, the writer is using Big-Endian while the system is Little-Endian
        let magic_bytes = &stream[0..4];
        let magic_val = u32::from_ne_bytes(magic_bytes.try_into().unwrap());
        assert_eq!(
            magic_val, 
            crate::MAGIC_VAL, 
            "Magic value byte order mismatch in stream"
        );

        // Step 3: Read back from stream using framing logic
        let mut reader = Cursor::new(stream);
        let recovered = read_from_stream(&mut reader)
            .await
            .expect("Failed to read from stream");

        // Step 4: Validate integrity
        assert_eq!(recovered.as_slice(), b"endian-test-data");
        assert_eq!(recovered.wire_format(), 0xAA);
        assert_eq!(recovered.version(), 0xBB);
        assert_eq!(recovered.error_version(), 0xCC);
        assert_eq!(recovered.status(), 0xDD);
    }

    #[tokio::test]
    async fn test_read_empty_payload() {
        let mut original = BridgeVec::with_capacity(0);
        original.set_status(1);
        original.set_error_version(5);
        
        let mut stream = Vec::new();
        write_to_stream(&mut stream, &original).await.unwrap();

        let mut reader = Cursor::new(stream);
        let recovered = read_from_stream(&mut reader).await.unwrap();

        assert_eq!(recovered.len(), 0);
        assert_eq!(recovered.status(), 1);
        assert_eq!(recovered.error_version(), 5);
    }

    #[tokio::test]
    async fn test_read_interrupted_header() {
        // Provide only 8 bytes of the required 16-byte header
        let partial_header = vec![0u8; 8];
        let mut reader = Cursor::new(partial_header);

        let result = read_from_stream(&mut reader).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::UnexpectedEof);
    }
}