//! Errors for the buffer family, and the error-payload encoding.
//!
//! An error reply is not a special channel — it is a normal message whose
//! payload is a JSON [`CapturedError`], with the header's `status` byte giving
//! a coarse class: what the caller should do next
//! ([docs/rpc-protocol.md §5](../../docs/rpc-protocol.md)). That is the entire
//! error path: no out-of-band signalling.
//!
//! *Where* an error came from is not in the status byte. The reply's
//! `trace_id`/`parent_span` name the failed call, and the [`CapturedError`]
//! carries the span that raised it ([`CapturedError::origin_span`]) and the
//! fine-grained detail ([`CapturedError::detail`]).
//!
//! Note on scope: [docs/error.rs](../../docs/error.rs) has a larger `PyroError`
//! with a `captured`/`LibraryInfo` stack that is not yet ported into this
//! crate. This is the focused subset the buffer family and header need, with
//! the same status mapping, so the wider port stays additive.

use std::fmt;
use std::panic::Location;

use crate::buf::header::{DataStatus, ParseError};
use crate::errors::WireError;

/// A failure, where it was raised, and optionally a backtrace — serialized as
/// JSON into an error message's payload.
///
/// To an untrusted peer, send only `message` and `origin_span`
/// ([docs/rpc-protocol.md §5.3](../../docs/rpc-protocol.md)): the rest stays
/// in your own trace store, where the span id finds it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CapturedError {
    pub message: String,
    /// The span that raised the error: the callee's own span, not the
    /// caller's `parent_span`. Hex on the wire, as in W3C `traceparent`, so
    /// JSON readers without 64-bit integers don't round it.
    #[serde(default, skip_serializing_if = "Option::is_none", with = "hex_span")]
    pub origin_span: Option<u64>,
    /// Fine-grained cause for logs and humans, e.g. `"schema_mismatch"` or
    /// `"utf8"`. Not a contract: callers branch on the status class.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backtrace: Option<String>,
}

impl CapturedError {
    pub fn new<M: fmt::Display>(message: M) -> Self {
        Self {
            message: message.to_string(),
            origin_span: None,
            detail: None,
            file: None,
            line: None,
            backtrace: None,
        }
    }

    #[track_caller]
    pub fn here<M: fmt::Display>(message: M) -> Self {
        Self::new(message).with_location(Location::caller())
    }

    pub fn with_location(mut self, loc: &Location<'_>) -> Self {
        self.file = Some(loc.file().to_string());
        self.line = Some(loc.line());
        self
    }

    pub fn with_backtrace(mut self, bt: impl fmt::Display) -> Self {
        self.backtrace = Some(bt.to_string());
        self
    }

    pub fn with_origin_span(mut self, span: u64) -> Self {
        self.origin_span = Some(span);
        self
    }

    pub fn with_detail(mut self, detail: impl fmt::Display) -> Self {
        self.detail = Some(detail.to_string());
        self
    }

    /// Only what an untrusted peer may see: the message and the span to look
    /// the rest up by.
    pub fn redacted(&self) -> Self {
        Self {
            message: self.message.clone(),
            origin_span: self.origin_span,
            detail: None,
            file: None,
            line: None,
            backtrace: None,
        }
    }
}

/// `Option<u64>` as a 16-digit lowercase hex string.
mod hex_span {
    use serde::{Deserialize, Deserializer, Serializer, de::Error};

    pub fn serialize<S: Serializer>(v: &Option<u64>, s: S) -> Result<S::Ok, S::Error> {
        match v {
            Some(span) => s.serialize_str(&format!("{span:016x}")),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<u64>, D::Error> {
        Option::<String>::deserialize(d)?
            .map(|h| u64::from_str_radix(&h, 16).map_err(D::Error::custom))
            .transpose()
    }
}

impl fmt::Display for CapturedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)?;
        if let Some(detail) = &self.detail {
            write!(f, " [{detail}]")?;
        }
        if let Some(span) = self.origin_span {
            write!(f, " (span {span:016x})")?;
        }
        if let (Some(file), Some(line)) = (&self.file, self.line) {
            write!(f, " (at {file}:{line})")?;
        }
        Ok(())
    }
}

/// The error type for buffer, header, and FFI operations.
#[derive(Debug, thiserror::Error)]
pub enum PyroError {
    /// A write would exceed a fixed-capacity buffer. Not a panic, not a grow,
    /// not a silent truncation ([docs/vec-plan.md §4](../../docs/vec-plan.md)).
    #[error("write of {need} bytes exceeds fixed capacity of {have}")]
    CapacityExceeded { need: usize, have: usize },

    /// The peer's schema hash disagrees with the expected one.
    #[error("schema mismatch: expected {expected:#018x}, got {got:#018x}")]
    SchemaMismatch { expected: u64, got: u64 },

    /// `capacity: u32` caps a message at 4 GiB, matching the header's `u32`
    /// length. A hard limit, reported rather than truncated.
    #[error("message of {0} bytes exceeds the 4 GiB limit imposed by the header's u32 length")]
    MessageTooLarge(usize),

    #[error("null pointer")]
    NullPointer,

    #[error("bad header: {0}")]
    Header(#[from] ParseError),

    /// A header arriving across FFI failed validation — critical, because it
    /// means the peer's view of the layout disagrees with ours.
    #[error("bad header in an FFI context: {0}")]
    HeaderFfi(CapturedError),

    /// The callee's code panicked; caught at the boundary, never unwound across
    /// the ABI.
    #[error("code panic: {0}")]
    CodePanic(CapturedError),

    /// A decoded error reply, tagged with the class it arrived under. Its
    /// origin is `captured.origin_span`.
    #[error("{status:?}: {captured}")]
    Remote {
        status: DataStatus,
        captured: CapturedError,
    },

    /// A decode was attempted against a status that does not carry data.
    #[error("payload is not decodable data: status {0:?}")]
    IncorrectParse(DataStatus),

    /// A `MessageWriter`/`MessageReader` operation failed at the wire-codec
    /// level (schema/inode mismatch, malformed data) rather than at the
    /// buffer level.
    #[error("wire codec error: {0}")]
    Wire(#[from] WireError),

    /// A `Domain` size class is fully checked out and not growable further —
    /// [docs/io-plan.md §4.2](../../docs/io-plan.md), `ReserveError::Empty`.
    #[error("domain pool exhausted for size class {class}")]
    PoolExhausted { class: usize },

    /// A blocking reserve gave up after its timeout without a chunk freeing.
    #[error("reserve timed out after {0:?} waiting for a free chunk")]
    ReserveTimeout(std::time::Duration),

    /// Requested length exceeds every size class the domain will grow to —
    /// `ReserveError::TooLarge`. Not a back-pressure condition; not retryable.
    #[error("requested {requested} bytes exceeds the domain's max class of {max}")]
    TooLarge { requested: usize, max: usize },
}

impl PyroError {
    #[track_caller]
    pub fn capacity_exceeded(need: usize, have: usize) -> Self {
        Self::CapacityExceeded { need, have }
    }

    #[track_caller]
    pub fn schema_mismatch(expected: u64, got: u64) -> Self {
        Self::SchemaMismatch { expected, got }
    }

    /// The error class this travels under: what the caller should do next.
    pub fn status(&self) -> DataStatus {
        match self {
            PyroError::SchemaMismatch { .. }
            | PyroError::MessageTooLarge(_)
            | PyroError::Wire(_)
            | PyroError::TooLarge { .. } => DataStatus::Rejected,
            PyroError::PoolExhausted { .. } | PyroError::ReserveTimeout(_) => {
                DataStatus::Unavailable
            }
            PyroError::CapacityExceeded { .. }
            | PyroError::NullPointer
            | PyroError::IncorrectParse(_) => DataStatus::Internal,
            PyroError::Header(_) | PyroError::HeaderFfi(_) => DataStatus::Protocol,
            PyroError::CodePanic(_) => DataStatus::Failed,
            PyroError::Remote { status, .. } => *status,
        }
    }

    /// The fine-grained cause recorded in [`CapturedError::detail`].
    pub fn detail(&self) -> &'static str {
        match self {
            PyroError::CapacityExceeded { .. } => "capacity_exceeded",
            PyroError::SchemaMismatch { .. } => "schema_mismatch",
            PyroError::MessageTooLarge(_) => "message_too_large",
            PyroError::NullPointer => "null_pointer",
            PyroError::Header(_) => "invalid_header",
            PyroError::HeaderFfi(_) => "invalid_header_ffi",
            PyroError::CodePanic(_) => "code_panic",
            PyroError::Remote { .. } => "remote",
            PyroError::IncorrectParse(_) => "incorrect_parse",
            PyroError::Wire(_) => "wire",
            PyroError::PoolExhausted { .. } => "pool_exhausted",
            PyroError::ReserveTimeout(_) => "reserve_timeout",
            PyroError::TooLarge { .. } => "too_large",
        }
    }

    /// The `CapturedError` that travels in the payload.
    #[track_caller]
    pub fn captured(&self) -> CapturedError {
        match self {
            PyroError::HeaderFfi(e) | PyroError::CodePanic(e) => e.clone(),
            PyroError::Remote { captured, .. } => captured.clone(),
            other => CapturedError::new(other)
                .with_detail(other.detail())
                .with_location(Location::caller()),
        }
    }

    /// Reconstruct from a decoded payload plus the class it arrived under.
    pub fn from_payload(status: DataStatus, source: CapturedError) -> Self {
        match status {
            DataStatus::Failed => PyroError::CodePanic(source),
            DataStatus::Protocol => PyroError::HeaderFfi(source),
            status => PyroError::Remote {
                status,
                captured: source,
            },
        }
    }
}

impl From<std::io::Error> for PyroError {
    #[track_caller]
    fn from(err: std::io::Error) -> Self {
        PyroError::Remote {
            status: DataStatus::Unavailable,
            captured: CapturedError::new(err)
                .with_detail("io")
                .with_location(Location::caller()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captured_error_round_trips_through_json() {
        let e = CapturedError::here("boom")
            .with_backtrace("frame0\nframe1")
            .with_origin_span(0x00f0_67aa_0ba9_02b7)
            .with_detail("schema_mismatch");
        let json = serde_json::to_string(&e).unwrap();
        assert!(
            json.contains(r#""origin_span":"00f067aa0ba902b7""#),
            "{json}"
        );
        let back: CapturedError = serde_json::from_str(&json).unwrap();
        assert_eq!(e, back);
    }

    #[test]
    fn redacted_keeps_only_message_and_span() {
        let e = CapturedError::here("boom")
            .with_backtrace("frame0")
            .with_origin_span(7)
            .with_detail("wire");
        let r = e.redacted();
        assert_eq!(r.message, "boom");
        assert_eq!(r.origin_span, Some(7));
        assert_eq!(
            (r.detail, r.file, r.line, r.backtrace),
            (None, None, None, None)
        );
    }

    #[test]
    fn every_status_maps_to_an_error_and_back() {
        for status in DataStatus::ALL.iter().copied() {
            if status.is_ok() {
                continue;
            }
            let err = PyroError::from_payload(status, CapturedError::new("x"));
            assert_eq!(
                err.status(),
                status,
                "status {status:?} did not survive the round trip"
            );
        }
    }
}
