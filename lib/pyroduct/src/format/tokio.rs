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
    let len = u32::from_le_bytes(header_buf[0..4].try_into().unwrap()) as usize;

    let client_id = u32::from_le_bytes(header_buf[4..8].try_into().unwrap());

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
    unsafe { vec.set_len(len as u32) };
    vec.set_client_id(client_id);
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
            RequestInner::View(v) => v.clone(),
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


    // 0x00: Length
    dest.write_u32_le(request.view().len() as u32).await?;

    // Wire header: matches the 16-byte layout that read_from_stream expects
    // 0x04: client_id
    dest.write_u32_le(request.client_id()).await?;

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
        header::{PyroHeader, PyroHeaderMut, DataStatus},
    };
    use std::io::Cursor;

    // ── Helpers ───────────────────────────────────────────────────────────────

    /// Write a `Request` to a `Vec`, then read it back via `read_from_stream`.
    /// Returns the recovered `PyroVec`.
    async fn roundtrip(request: &Request) -> PyroVec {
        let mut stream = Vec::new();
        write_to_stream(&mut stream, request, None).await.unwrap();
        let mut reader = Cursor::new(stream);
        let mut recovered = PyroVec::with_capacity(0);
        read_from_stream(&mut reader, None, &mut recovered)
            .await
            .unwrap();
        recovered
    }

    fn make_request(data: &[u8]) -> Request {
        let mut vec = PyroVec::with_capacity(data.len());
        vec.extend_from_slice(data);
        vec.into()
    }

    /// Build a Request where:
    /// - `override_*` fields, if `Some`, override the value in the wire format
    /// - `vec_*` fields set the underlying PyroVec (used as fallback when override is None)
    fn make_request_with(
        data: &[u8],
        vec_class_id: u8,
        vec_fn_id: u8,
        vec_mux_id: u32,
        vec_client_id: u32,
        override_class_id: Option<u8>,
        override_fn_id: Option<u8>,
        override_mux_id: Option<u32>,
        override_client_id: Option<u32>,
    ) -> Request {
        let mut vec = PyroVec::with_capacity(data.len());
        vec.extend_from_slice(data);
        vec.set_class_id(vec_class_id);
        vec.set_fn_id(vec_fn_id);
        vec.set_mux_id(vec_mux_id);
        vec.set_client_id(vec_client_id);
        let mut request: Request = vec.into();
        if let Some(v) = override_class_id { request.class_id = Some(v); }
        if let Some(v) = override_fn_id { request.fn_id = Some(v); }
        if let Some(v) = override_mux_id { request.mux_id = Some(v); }
        if let Some(v) = override_client_id { request.client_id = Some(v); }
        request
    }

    // ── Roundtrip / integrity ─────────────────────────────────────────────────

    #[tokio::test]
    async fn test_roundtrip_basic_payload() {
        let mut vec = PyroVec::with_capacity(16);
        vec.extend_from_slice(b"endian-test-data");
        let request = vec.into();

        let recovered = roundtrip(&request).await;
        assert_eq!(recovered.as_slice(), b"endian-test-data");
    }

    #[tokio::test]
    async fn test_roundtrip_empty_payload() {
        let mut vec = PyroVec::with_capacity(0);
        vec.set_status(DataStatus::Empty);
        vec.set_class_id(5);
        let request = vec.into();

        let recovered = roundtrip(&request).await;
        assert_eq!(recovered.len(), 0);
        assert_eq!(recovered.status(), Ok(DataStatus::Empty));
        assert_eq!(recovered.class_id(), 5);
    }

    #[tokio::test]
    async fn test_roundtrip_all_header_fields() {
        let mut vec = PyroVec::with_capacity(16);
        vec.extend_from_slice(b"full-header");
        vec.set_wire_format(0xAA);
        vec.set_class_id(0xBB);
        vec.set_fn_id(0xCC);
        vec.set_status(DataStatus::LocalIo);
        let request = vec.into();

        let recovered = roundtrip(&request).await;

        assert_eq!(recovered.wire_format(), 0xAA);
        assert_eq!(recovered.class_id(), 0xBB);
        assert_eq!(recovered.fn_id(), 0xCC);
        assert_eq!(recovered.status(), Ok(DataStatus::LocalIo));
        // mux_id defaults to 0 in the view (no mux_id set), so that's what we get back
        assert_eq!(recovered.mux_id(), 0);
    }

    // ── Request field overrides ───────────────────────────────────────────────

    /// When a Request field is `Some`, it overrides the PyroView's value.
    /// When `None`, the PyroView's value is used.

    #[tokio::test]
    async fn test_override_class_id() {
        // Override with Some
        let request = make_request_with(b"test", 0x11, 0, 0, 0, Some(0xFF), None, None, None);
        let recovered = roundtrip(&request).await;
        assert_eq!(recovered.class_id(), 0xFF, "Override value should win");

        // Fall back to view (None)
        let request = make_request_with(b"test", 0x11, 0, 0, 0, None, None, None, None);
        let recovered = roundtrip(&request).await;
        assert_eq!(recovered.class_id(), 0x11, "View value should be used when Some is absent");
    }

    #[tokio::test]
    async fn test_override_fn_id() {
        let request = make_request_with(b"test", 0, 0x22, 0, 0, None, Some(0xEE), None, None);
        let recovered = roundtrip(&request).await;
        assert_eq!(recovered.fn_id(), 0xEE);

        let request = make_request_with(b"test", 0, 0x22, 0, 0, None, None, None, None);
        let recovered = roundtrip(&request).await;
        assert_eq!(recovered.fn_id(), 0x22);
    }

    #[tokio::test]
    async fn test_override_mux_id() {
        let request = make_request_with(b"test", 0, 0, 0xAAAAAAAA, 0, None, None, Some(0xDEADBEEF), None);
        let recovered = roundtrip(&request).await;
        assert_eq!(recovered.mux_id(), 0xDEADBEEF);

        let request = make_request_with(b"test", 0, 0, 0xAAAAAAAA, 0, None, None, None, None);
        let recovered = roundtrip(&request).await;
        assert_eq!(recovered.mux_id(), 0xAAAAAAAA);
    }

    #[tokio::test]
    async fn test_override_client_id() {
        let request = make_request_with(b"test", 0, 0, 0, 0x12345678, None, None, None, Some(0xFEDCBA98));
        let recovered = roundtrip(&request).await;
        assert_eq!(recovered.client_id(), 0xFEDCBA98);

        let request = make_request_with(b"test", 0, 0, 0, 0x12345678, None, None, None, None);
        let recovered = roundtrip(&request).await;
        assert_eq!(recovered.client_id(), 0x12345678);
    }

    #[tokio::test]
    async fn test_override_all_fields_at_once() {
        // vec_* values are the fallback; override_* values should win
        let request = make_request_with(
            b"alloverride", 0x11, 0x22, 0xAAAAAAAA, 0xBBBBBBBB,
            Some(0xDD), Some(0xEE), Some(0x11223344), Some(0xCCCCCCCC),
        );

        let recovered = roundtrip(&request).await;

        assert_eq!(recovered.client_id(), 0xCCCCCCCC);
        assert_eq!(recovered.class_id(), 0xDD);
        assert_eq!(recovered.fn_id(), 0xEE);
        assert_eq!(recovered.mux_id(), 0x11223344);
        // Non-overridden fields should come from the view
        assert_eq!(recovered.status(), Ok(DataStatus::RemoteUtf8));
        assert_eq!(recovered.wire_format(), 0x55);
    }

    // ── RequestInner::View vs RequestInner::Vec ───────────────────────────────

    #[tokio::test]
    async fn test_roundtrip_from_view() {
        let mut vec = PyroVec::with_capacity(6);
        vec.extend_from_slice(b"viewed");
        vec.set_class_id(0x42);
        let request: Request = vec.view().into();

        let recovered = roundtrip(&request).await;
        assert_eq!(recovered.as_slice(), b"viewed");
        assert_eq!(recovered.class_id(), 0x42);
    }

    #[tokio::test]
    async fn test_roundtrip_from_vec() {
        let mut vec = PyroVec::with_capacity(6);
        vec.extend_from_slice(b"vecced");
        vec.set_class_id(0x99);
        let request: Request = vec.into();

        let recovered = roundtrip(&request).await;
        assert_eq!(recovered.as_slice(), b"vecced");
        assert_eq!(recovered.class_id(), 0x99);
    }

    // ── Error cases ───────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_read_rejects_bad_magic() {
        let mut bad_packet = Vec::new();
        bad_packet.extend_from_slice(&0xDEADBEEFu32.to_ne_bytes()); // BAD Magic
        bad_packet.extend_from_slice(&10u32.to_le_bytes()); // Len
        bad_packet.extend_from_slice(&[0u8; 8]);
        bad_packet.extend_from_slice(&[0u8; 10]);

        let mut cursor = Cursor::new(bad_packet);
        let mut recovered = PyroVec::with_capacity(0);
        let result = read_from_stream(&mut cursor, None, &mut recovered).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::InvalidData);
    }

    #[tokio::test]
    async fn test_read_detects_header_eof() {
        let partial_header = vec![0u8; 10];
        let mut cursor = Cursor::new(partial_header);
        let mut recovered = PyroVec::with_capacity(0);
        let result = read_from_stream(&mut cursor, None, &mut recovered).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::UnexpectedEof);
    }

    #[tokio::test]
    async fn test_read_detects_body_eof() {
        let body_len: u32 = 10;
        let mut packet = Vec::new();
        packet.extend_from_slice(&0x7079726Fu32.to_ne_bytes()); // "pyro"
        packet.extend_from_slice(&body_len.to_le_bytes());
        packet.extend_from_slice(&[0u8; 8]);
        packet.extend_from_slice(b"12345"); // Only 5 bytes, promised 10

        let mut cursor = Cursor::new(packet);
        let mut recovered = PyroVec::with_capacity(0);
        let result = read_from_stream(&mut cursor, None, &mut recovered).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::UnexpectedEof);
    }

    #[tokio::test]
    async fn test_read_too_large() {
        let config = PyroStreamSettings {
            max_msg_size: 5,
            timeout: Duration::from_secs(1),
        };
        let mut packet = Vec::new();
        packet.extend_from_slice(&0x7079726Fu32.to_ne_bytes());
        packet.extend_from_slice(&20u32.to_le_bytes()); // 20 > max 5
        packet.extend_from_slice(&[0u8; 8]);
        packet.extend_from_slice(&[0u8; 20]);

        let mut cursor = Cursor::new(packet);
        let mut recovered = PyroVec::with_capacity(0);
        let result = read_from_stream(&mut cursor, Some(&config), &mut recovered).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::InvalidData);
    }
}
