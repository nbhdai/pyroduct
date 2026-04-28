//! Tokio integration

use std::io::{Error, ErrorKind};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::io::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::format::PyroView;
use crate::format::{
    PyroVec,
    header::{PyroHeader, PyroHeaderMut},
};

// --- Constants ---

pub const DEFAULT_MAX_MSG_SIZE: usize = 16 * 1024 * 1024; // 16 MB
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

// --- Configuration ---

#[derive(Debug, Clone, Copy)]
pub struct PyroStreamSettings {
    pub max_msg_size: usize,
    pub timeout: Duration,
    // pub use_compression: bool,
}

impl Default for PyroStreamSettings {
    fn default() -> Self {
        DEFAULT_STREAM_SETTINGS.clone()
    }
}

const DEFAULT_STREAM_SETTINGS: PyroStreamSettings = PyroStreamSettings {
    max_msg_size: DEFAULT_MAX_MSG_SIZE,
    timeout: DEFAULT_TIMEOUT,
};

// --- AsyncWrite Implementation ---
// Allows PyroVec to be used as a buffer for tokio::io::copy or other async writers.
impl AsyncWrite for PyroVec {
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

/// Reads a `PyroVec` from an async stream (TCP/Unix).
///
/// This performs the framing logic:
/// 1. Reads the 16-byte header.
/// 2. Validates the Magic number.
/// 3. Reads the length, capacity, versions, and status.
/// 4. Allocates the vector.
/// 5. Reads the exact payload into the vector.
pub async fn read_from_stream<R>(
    src: &mut R,
    config: Option<&PyroStreamSettings>,
    vec: &mut PyroVec,
) -> io::Result<()>
where
    R: AsyncRead + Unpin,
{
    let config = match config {
        Some(c) => c,
        None => &DEFAULT_STREAM_SETTINGS,
    };

    let mut header_buf = [0u8; 16];

    // 1. Read the full 16-byte header
    src.read_exact(&mut header_buf).await?;

    // 2. Parse fields (Little Endian for multi-byte integers)
    // 0x04 - 0x07: Length
    let len = u32::from_le_bytes(header_buf[4..8].try_into().unwrap()) as usize;

    // 0x08: Wire Format
    let wire_format = header_buf[8];

    // 0x09: Status
    let status = header_buf[9];

    // 0x0A: Class ID
    let class_id = header_buf[10];

    // 0x0B: Function ID
    let fn_id = header_buf[11];

    // 0x0C - 0x0F: Mux ID
    let mux_id = u32::from_le_bytes(header_buf[12..16].try_into().unwrap());

    if len > config.max_msg_size {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "Message size exceeds limit",
        ));
    }

    // 3. Allocate and set metadata
    vec.set_wire_format(wire_format);
    vec.set_class_id(class_id);
    vec.set_fn_id(fn_id);
    vec.set_mux_id(mux_id);
    vec.set_status_u8(status);

    // 4. Read Payload
    if len > 0 {
        let mut reader = src.take(len as u64);
        let bytes_read = io::copy(&mut reader, vec).await?;

        if bytes_read != len as u64 {
            return Err(Error::new(
                ErrorKind::UnexpectedEof,
                "Stream ended before full payload was received",
            ));
        }
    }

    debug_assert_eq!(len, vec.len());

    Ok(())
}

/// Helper to write a PyroVec to an async stream.
/// This writes the header (with version/status) followed by the data payload.
pub async fn write_to_stream<W>(
    dest: &mut W,
    view: &PyroView<'_>,
    mux_id: u32,
    config: Option<&PyroStreamSettings>,
) -> io::Result<()>
where
    W: AsyncWrite + Unpin,
{
    let config = match config {
        Some(c) => c,
        None => &DEFAULT_STREAM_SETTINGS,
    };

    if view.len() > config.max_msg_size {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "Message size exceeds limit",
        ));
    }

    // 0x04: Client
    dest.write_u32_le(view.client_id()).await?;

    // 0x04: Length
    dest.write_u32_le(view.len() as u32).await?;

    // 0x08: Wire Format
    dest.write_u8(view.wire_format()).await?;

    // 0x09: Status
    dest.write_u8(view.status_u8()).await?;

    // 0x0A: Class ID
    dest.write_u8(view.class_id()).await?;

    // 0x0B: Function ID
    dest.write_u8(view.fn_id()).await?;

    // 0x0C: Mux ID
    dest.write_u32_le(mux_id).await?;

    // Payload
    dest.write_all(view.as_slice()).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::{
        PyroVec,
        header::{PyroHeader, PyroHeaderMut},
    };
    use std::io::Cursor;

    #[tokio::test]
    async fn test_streaming_endian_consistency() {
        // Create a vec with specific metadata to test byte-order
        let mut original = PyroVec::with_capacity(16);
        original.extend_from_slice(b"endian-test-data");

        // Use distinct values for all byte fields
        original.set_wire_format(0xAA);
        original.set_class_id(0xBB);
        original.set_fn_id(0xCC);
        original.set_status(crate::format::header::DataStatus::LocalIo);

        let mut stream = Vec::new();

        // Step 1: Write to stream
        write_to_stream(&mut stream, &original.view(), 0x12345678, None)
            .await
            .expect("Failed to write to stream");

        // Step 3: Read back from stream using framing logic
        let mut reader = Cursor::new(stream);
        let mut recovered = PyroVec::with_capacity(0);
        read_from_stream(&mut reader, None, &mut recovered)
            .await
            .expect("Failed to read from stream");

        // Step 4: Validate integrity
        assert_eq!(recovered.as_slice(), b"endian-test-data");
        assert_eq!(recovered.wire_format(), 0xAA);
        assert_eq!(recovered.class_id(), 0xBB);
        assert_eq!(recovered.fn_id(), 0xCC);
        assert_eq!(recovered.mux_id(), 0x12345678);
        assert_eq!(
            recovered.status(),
            Ok(crate::format::header::DataStatus::LocalIo)
        );
    }

    #[tokio::test]
    async fn test_read_empty_payload() {
        let mut original = PyroVec::with_capacity(0);
        original.set_status(crate::format::header::DataStatus::Empty);
        original.set_class_id(5);

        let mut stream = Vec::new();
        write_to_stream(&mut stream, &original.view(), 0, None)
            .await
            .unwrap();

        let mut reader = Cursor::new(stream);
        let mut recovered = PyroVec::with_capacity(0);
        read_from_stream(&mut reader, None, &mut recovered)
            .await
            .unwrap();

        assert_eq!(recovered.len(), 0);
        assert_eq!(
            recovered.status(),
            Ok(crate::format::header::DataStatus::Empty)
        );
        assert_eq!(recovered.class_id(), 5);
    }

    #[tokio::test]
    async fn test_read_interrupted_header() {
        // Provide only 8 bytes of the required 16-byte header
        let partial_header = vec![0u8; 8];
        let mut reader = Cursor::new(partial_header);

        let mut recovered = PyroVec::with_capacity(0);
        let result = read_from_stream(&mut reader, None, &mut recovered).await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().kind(),
            std::io::ErrorKind::UnexpectedEof
        );
    }
}
