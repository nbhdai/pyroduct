//! The error path: an error reply is a normal message whose payload is a JSON
//! [`CapturedError`], discriminated by the header's status byte.
//!
//! No separate error channel, no out-of-band signalling. Ported from
//! [docs/encoding.rs](../../docs/encoding.rs) onto the §3 header.

use crate::buf::error::{CapturedError, PyroError};
use crate::buf::header::{DataStatus, PyroHeader, PyroParser};
use crate::buf::{PyroBuf, PyroRef, PyroVec};

impl CapturedError {
    /// Serialize into a growable, self-owned buffer.
    ///
    /// Deliberately the **unoptimized path**: the size is not knowable in
    /// advance, and this is produced precisely when things are going wrong. Its
    /// critical property is independence from any pool's free lists — the error
    /// you most need to deliver is "the pool is exhausted".
    pub fn encode(&self) -> PyroVec {
        // Serializing to a Vec first keeps the fallible step out of the buffer:
        // a serializer failure here would otherwise leave a half-written
        // payload with a stale length.
        let json = serde_json::to_vec(self).unwrap_or_else(|e| {
            // A CapturedError is plain strings; failure means an allocator or
            // invariant problem, so fall back to a message that cannot fail.
            format!("{{\"message\":\"error serialization failed: {e}\"}}").into_bytes()
        });
        let mut v = PyroVec::with_capacity(json.len());
        v.extend_from_slice(&json);
        v
    }
}

impl PyroError {
    /// Encode as a sealed error message: JSON payload, error-class status, and
    /// `schema_hash` zero — an error payload matches no schema. The caller
    /// stamps `trace_id`/`parent_span` to echo the failed call.
    ///
    /// Users should normally let the FFI/transport glue do this.
    #[doc(hidden)]
    #[track_caller]
    pub fn encode(&self) -> PyroBuf {
        let status = self.status();
        let vec = self.captured().encode();
        // Hash zero plus a nonzero length and a non-Empty status: not the unit
        // message, and not claiming a schema either.
        let mut buf = vec.seal(0);
        buf.set_status(status);
        buf
    }
}

impl<'a> PyroRef<'a> {
    /// Decode this message's error payload, if it carries one.
    ///
    /// Returns `Ok(())` when the status says the payload is data. Checking
    /// status first means an error reply becomes an `Err` before any decode is
    /// attempted.
    pub fn check_status(&self) -> Result<(), PyroError> {
        let status = match self.status() {
            Ok(s) => s,
            // An unknown status from a newer peer is not something to guess at.
            Err(raw) => {
                return Err(PyroError::HeaderFfi(CapturedError::new(format!(
                    "unknown status code {raw}"
                ))));
            }
        };
        if status.is_ok() {
            return Ok(());
        }
        // Errors only from here on.
        let captured = self.captured_error().unwrap_or_else(|| {
            CapturedError::new(format!("{status:?} with an undecodable error payload"))
        });
        Err(PyroError::from_payload(status, captured))
    }

    /// The JSON `CapturedError` in the payload, if it parses.
    pub fn captured_error(&self) -> Option<CapturedError> {
        if self.payload().is_empty() {
            return None;
        }
        serde_json::from_slice(self.payload()).ok()
    }

    /// Compare the peer's schema hash against a locally known one, honouring
    /// the unit-message exemption.
    pub fn verify_schema(&self, expected: u64) -> Result<(), PyroError> {
        match self.check_schema(expected) {
            Ok(()) => Ok(()),
            Err((expected, got)) => Err(PyroError::SchemaMismatch { expected, got }),
        }
    }
}

/// Validate a status byte, for callers holding raw header bytes.
pub fn parse_status(raw: u8) -> Result<DataStatus, crate::buf::header::ParseError> {
    DataStatus::parse(raw).map_err(crate::buf::header::ParseError::UnknownStatus)
}

/// The reserved byte is validated zero everywhere, including here — re-exported
/// so the offset has exactly one definition.
pub const OFFSET_RESERVED: usize = PyroParser::OFFSET_RESERVED;
