//! Tokio intergration

use tokio::io::{self, AsyncRead, AsyncReadExt, AsyncWrite};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::io::{Error, ErrorKind};

use crate::LenAlignedVec;

// --- AsyncWrite Implementation ---
// Allows LenAlignedVec to be used as a buffer for tokio::io::copy or other async writers.
impl AsyncWrite for LenAlignedVec {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        // We can access public methods here.
        self.extend_from_slice(buf);
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        // No-op for in-memory vector
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        // No-op for in-memory vector
        Poll::Ready(Ok(()))
    }
}

// --- Framing Helpers ---

/// Reads a `LenAlignedVec` from an async stream (TCP/Unix).
/// 
/// This performs the framing logic:
/// 1. Reads the 16-byte header.
/// 2. Validates the Magic number.
/// 3. Reads the length.
/// 4. Allocates the vector.
/// 5. Reads the exact payload into the vector.
pub async fn read_from_stream<R>(src: &mut R) -> io::Result<LenAlignedVec> 
where 
    R: AsyncRead + Unpin
{
    let mut header_buf = [0u8; 16];
    
    // 1. Read Header (16 bytes)
    src.read_exact(&mut header_buf).await?;

    let magic = u32::from_ne_bytes(header_buf[0..4].try_into().unwrap());
    let len = u32::from_ne_bytes(header_buf[4..8].try_into().unwrap()) as usize;

    // 3. Validate Magic (0x7079726F / "pyro")
    if magic != LenAlignedVec::MAGIC_VAL {
        return Err(Error::new(ErrorKind::InvalidData, "Invalid LenAlignedVec magic header"));
    }

    let mut vec = LenAlignedVec::with_capacity(len);

    if len > 0 {
        let mut reader = src.take(len as u64);
        let bytes_read = io::copy(&mut reader, &mut vec).await?;
        
        if bytes_read != len as u64 {
            return Err(Error::new(ErrorKind::UnexpectedEof, "Stream ended before full payload was received"));
        }
    }

    Ok(vec)
}

/// Helper to write a LenAlignedVec to an async stream.
/// This writes the raw allocation (Header + Data) directly.
pub async fn write_to_stream<W>(dest: &mut W, vec: &LenAlignedVec) -> io::Result<()>
where
    W: AsyncWrite + Unpin
{
    
    // Reconstruct Header for the wire
    let magic = 0x7079726Fu32;
    let len = vec.len() as u32;
    let cap = vec.capacity() as u32;

    use tokio::io::AsyncWriteExt;

    dest.write_u32(magic).await?; // 0..4
    dest.write_u32(len).await?;   // 4..8
    dest.write_u32(cap).await?;   // 8..12
    dest.write_u32(0).await?;     // 12..16 (Padding/Uninit)

    // Write Data
    dest.write_all(vec.as_slice()).await?;
    
    Ok(())
}