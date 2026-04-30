//! Tokio integration

use crate::format::PyroView;
use crate::format::{
    PyroVec,
    header::{PyroHeader, PyroHeaderMut},
};
use std::io::{Error, ErrorKind};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::io::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

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

pub enum RequestInner {
    View(PyroView),
    Vec(PyroVec),
}

impl From<PyroView> for RequestInner {
    fn from(value: PyroView) -> Self {
        RequestInner::View(value)
    }
}

impl From<PyroVec> for RequestInner {
    fn from(value: PyroVec) -> Self {
        RequestInner::Vec(value)
    }
}

pub struct Request {
    pub client_id: Option<u32>,
    pub class_id: Option<u8>,
    pub fn_id: Option<u8>,
    pub mux_id: Option<u32>,
    pub inner: RequestInner,
}

impl From<PyroView> for Request {
    fn from(value: PyroView) -> Self {
        Request {
            client_id: None,
            class_id: None,
            fn_id: None,
            mux_id: None,
            inner: RequestInner::View(value),
        }
    }
}

impl From<PyroVec> for Request {
    fn from(value: PyroVec) -> Self {
        Request {
            client_id: None,
            class_id: None,
            fn_id: None,
            mux_id: None,
            inner: RequestInner::Vec(value),
        }
    }
}

impl Request {
    fn view(&self) -> PyroView {
        match &self.inner {
            RequestInner::View(v) => *v,
            RequestInner::Vec(vec) => vec.view(),
        }
    }

    pub fn client_id(&self) -> u32 {
        self.client_id.unwrap_or_else(|| self.view().client_id())
    }

    pub fn class_id(&self) -> u8 {
        self.class_id.unwrap_or_else(|| self.view().class_id())
    }

    pub fn fn_id(&self) -> u8 {
        self.fn_id.unwrap_or_else(|| self.view().fn_id())
    }

    pub fn mux_id(&self) -> u32 {
        self.mux_id.unwrap_or_else(|| self.view().mux_id())
    }
}

/// Helper to write a PyroVec to an async stream.
/// This writes the header (with version/status) followed by the data payload.
pub async fn write_to_stream<W>(
    dest: &mut W,
    request: &Request,
    config: Option<&PyroStreamSettings>,
) -> io::Result<()>
where
    W: AsyncWrite + Unpin,
{
    let config = match config {
        Some(c) => c,
        None => &DEFAULT_STREAM_SETTINGS,
    };

    if request.view().len() > config.max_msg_size {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "Message size exceeds limit",
        ));
    }

    // 0x04: Client
    dest.write_u32_le(request.client_id()).await?;

    // 0x04: Length
    dest.write_u32_le(request.view().len() as u32).await?;

    // 0x08: Wire Format
    dest.write_u8(request.view().wire_format()).await?;

    // 0x09: Status
    dest.write_u8(request.view().status_u8()).await?;

    // 0x0A: Class ID
    dest.write_u8(request.class_id()).await?;

    // 0x0B: Function ID
    dest.write_u8(request.fn_id()).await?;

    // 0x0C: Mux ID
    dest.write_u32_le(request.mux_id()).await?;

    // Payload
    dest.write_all(request.view().as_slice()).await?;

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

        let mut request: Request = original.view().into();
        request.mux_id = Some(0x12345678);

        // Step 1: Write to stream
        write_to_stream(&mut stream, &request, None)
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
        let request = original.view().into();
        let mut stream = Vec::new();
        write_to_stream(&mut stream, &request, None).await.unwrap();

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

    #[tokio::test]
    async fn test_stream_roundtrip() {
        let mut original = PyroVec::with_capacity(32);
        original.extend_from_slice(b"Async Data Packet");
        original.set_status(DataStatus::RemoteUtf8);
        original.set_fn_id(2);

        let mut stream_buffer = Vec::new();

        write_to_stream(&mut stream_buffer, &original.view(), 0, None)
            .await
            .expect("Write failed");

        let mut cursor = Cursor::new(stream_buffer);
        let mut recovered = PyroVec::with_capacity(0);
        read_from_stream(&mut cursor, None, &mut recovered)
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
        write_to_stream(&mut stream_buffer, &original.view(), 0, None)
            .await
            .unwrap();

        let mut cursor = Cursor::new(stream_buffer);
        let mut recovered = PyroVec::with_capacity(0);
        read_from_stream(&mut cursor, None, &mut recovered)
            .await
            .unwrap();

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
        let mut recovered = PyroVec::with_capacity(0);

        let result = read_from_stream(&mut cursor, None, &mut recovered).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::InvalidData);
    }

    #[tokio::test]
    async fn test_read_detects_header_eof() {
        // Header is 16 bytes. We only provide 10.
        let partial_header = vec![0u8; 10];
        let mut cursor = Cursor::new(partial_header);

        let mut recovered = PyroVec::with_capacity(0);
        let result = read_from_stream(&mut cursor, None, &mut recovered).await;

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
        let mut recovered = PyroVec::with_capacity(0);
        let result = read_from_stream(&mut cursor, None, &mut recovered).await;

        assert!(result.is_err());
        // Should fail because it couldn't fill the buffer
        assert_eq!(
            result.unwrap_err().kind(),
            std::io::ErrorKind::UnexpectedEof
        );
    }
}
