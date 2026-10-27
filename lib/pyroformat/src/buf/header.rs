//! The 48-byte transmitted message header, and the schema fingerprint.
//!
//! Layout ([docs/vec-plan.md §3](../../docs/vec-plan.md)), every field at its
//! natural alignment:
//!
//! ```text
//! -- framing word ---------------------------------------------------------
//! offset  0..4   len           u32   payload length in bytes (excludes this header)
//! offset  4      version       u8    PROTOCOL_VERSION / wire format
//! offset  5      status        u8    DataStatus — message kind / error class
//! offset  6      flags         u8    bit0 sampled, 7 spare
//! offset  7      reserved      u8    must be zero; validated (§8.1)
//! -- schema ---------------------------------------------------------------
//! offset  8..16  schema_hash   u64   hash of the inode table this payload matches
//! -- tracing (W3C trace context) ------------------------------------------
//! offset 16..32  trace_id      u128  trace this message belongs to; 0 = none
//! offset 32..40  parent_span   u64   sender's span id, the receiver's parent
//! offset 40..48  timestamp     i64   ns since the Unix epoch (±292 years)
//! ```
//!
//! `trace_id`, `parent_span` and the sampled flag are exactly W3C
//! `traceparent`, so an OpenTelemetry context round-trips through the header
//! with nothing lost. When no context is active, [`super::next_trace_id`]
//! supplies a fresh `trace_id` and [`super::next_span`] the sender's span id.
//!
//! This header is point-to-point and schema-bound. The RPC protocol built on
//! it ([docs/rpc-protocol.md](../../docs/rpc-protocol.md)) routes calls by
//! `schema_hash` and multiplexes them by `parent_span`, scoped to one
//! authenticated connection, so there are no client/class/fn/mux fields here.

/// Wire format / protocol version stamped at [`PyroParser::OFFSET_VERSION`].
pub const PROTOCOL_VERSION: u8 = 1;

/// Transmitted header size. Also the payload's offset from the message pointer.
pub const HEADER_SIZE: usize = 48;

/// Every buffer, message pointer and payload is aligned to this.
///
/// Set by the reader: `check::check_alignment` validates *absolute* pointer
/// alignment up to 16 bytes, and `Timestamp` is a 16-byte class.
pub const ALIGN: usize = 16;

// ---------------------------------------------------------------------------
// Header flag bits (offset 6)
// ---------------------------------------------------------------------------

/// The trace is being recorded: W3C trace-flags bit 0.
pub const FLAG_SAMPLED: u8 = 1 << 0;

// ---------------------------------------------------------------------------
// DataStatus
// ---------------------------------------------------------------------------

macro_rules! define_data_status {
    ($( $(#[$m:meta])* $name:ident = $val:expr ),* $(,)?) => {
        /// Status byte at [`PyroParser::OFFSET_STATUS`]: what kind of message
        /// this is, and for an error reply, what the caller should do about it.
        ///
        /// See [docs/rpc-protocol.md §5](../../docs/rpc-protocol.md).
        ///
        /// | range | meaning | payload |
        /// | --- | --- | --- |
        /// | 0 | the unit message | none |
        /// | 1..=15 | message kinds | data, schema tables, an [`Allocation`], or none |
        /// | 16..=31 | error replies, by what the caller does next | JSON [`CapturedError`](super::CapturedError) |
        ///
        /// Error classes are deliberately coarse. *Where* an error came from is
        /// not encoded here: the reply's `trace_id`/`parent_span` say which call
        /// failed, and the [`CapturedError`](super::CapturedError) records the
        /// span that raised it and the fine-grained detail.
        #[repr(u8)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum DataStatus {
            $( $(#[$m])* $name = $val, )*
        }

        impl DataStatus {
            /// Parse a raw status byte. Unknown codes are returned verbatim;
            /// the protocol layer decides whether that closes the connection.
            pub const fn parse(i: u8) -> Result<DataStatus, u8> {
                match i {
                    $( $val => Ok(DataStatus::$name), )*
                    other => Err(other),
                }
            }

            /// Every variant, for exhaustive round-trip tests.
            pub const ALL: &'static [DataStatus] = &[ $( DataStatus::$name, )* ];

            /// Encode back to the wire byte.
            pub const fn to_u8(self) -> u8 {
                match self {
                    $( DataStatus::$name => $val, )*
                }
            }
        }

        impl From<DataStatus> for u8 {
            fn from(s: DataStatus) -> u8 {
                s.to_u8()
            }
        }
    };
}

define_data_status! {
    /// The unit message: no payload, no schema.
    Unit = 0,

    // --- Message kinds (1..=15) --------------------------------------------
    /// Standalone data outside any call: queues, files, pipes.
    Data = 1,
    /// Connection open: the sender's interface tables. Must be first.
    Hello = 2,
    /// A call. `schema_hash` is the function's call hash; `parent_span` is the
    /// caller-issued call id.
    Request = 3,
    /// A successful reply. Echoes the request's `trace_id` and `parent_span`.
    Reply = 4,
    /// Cancel the call named by `parent_span`. No payload.
    Cancel = 5,
    /// Graceful close: no new calls; in-flight calls finish.
    Close = 6,
    /// Connection-level: please change the largest message you accept from
    /// me to this [`Allocation`]. Ids 0.
    Allocate = 7,
    /// Connection-level: the largest message I now accept from you, as an
    /// [`Allocation`]. Answers exactly one `Allocate`; a refusal restates the
    /// old maximum. Ids 0.
    Grant = 8,

    // --- Error replies (16..=31), by what the caller does next --------------
    /// The callee's own code returned an error or panicked. Surface it.
    Failed = 16,
    /// The request was invalid: unknown function, schema mismatch, malformed
    /// or oversized payload. Don't retry it unchanged.
    Rejected = 17,
    /// The caller may not make this call.
    Denied = 18,
    /// Overloaded, out of buffers, or shutting down. Retry later or elsewhere.
    Unavailable = 19,
    /// The call was cancelled or its deadline passed.
    Cancelled = 20,
    /// A pyroformat/runtime fault at the callee. A bug; retrying may help.
    Internal = 21,
    /// The peer broke the protocol. The connection is closing.
    Protocol = 22,
}

impl DataStatus {
    /// Not an error.
    pub const fn is_ok(self) -> bool {
        self.to_u8() < 16
    }

    /// An error; the payload is a JSON `CapturedError`.
    pub const fn is_error(self) -> bool {
        !self.is_ok()
    }

    /// The payload is pyroformat data matching `schema_hash`.
    pub const fn carries_data(self) -> bool {
        matches!(
            self,
            DataStatus::Data | DataStatus::Request | DataStatus::Reply
        )
    }

    /// The payload is an [`Allocation`].
    pub const fn carries_allocation(self) -> bool {
        matches!(self, DataStatus::Allocate | DataStatus::Grant)
    }

    /// Sent by a call's caller. The receiver resolves it in its *incoming*
    /// call table.
    pub const fn flows_to_callee(self) -> bool {
        matches!(self, DataStatus::Request | DataStatus::Cancel)
    }

    /// Sent by a call's callee. The receiver resolves it in its *outgoing*
    /// call table.
    pub const fn flows_to_caller(self) -> bool {
        self.is_reply()
    }

    /// The final answer to a call: a reply or a call-level error. Exactly one
    /// per call. [`DataStatus::Protocol`] is connection-level, so it is not
    /// one.
    pub const fn is_reply(self) -> bool {
        matches!(self, DataStatus::Reply)
            || (self.is_error() && !matches!(self, DataStatus::Protocol))
    }

    /// Worth retrying, possibly on another node.
    pub const fn is_retryable(self) -> bool {
        matches!(self, DataStatus::Unavailable | DataStatus::Internal)
    }
}

// ---------------------------------------------------------------------------
// Allocation
// ---------------------------------------------------------------------------

/// The 16-byte payload of [`DataStatus::Allocate`] and [`DataStatus::Grant`]:
/// the largest message *payload* (`len`) the receiver accepts from this peer.
///
/// ```text
/// offset 0..4   max_len   u32  requested / now in effect
/// offset 4..16  reserved       must be zero
/// ```
///
/// It is a cap, not a reservation: each message is still received into the
/// smallest pool size class that fits its own `len`. See
/// [docs/rpc-protocol.md §7](../../docs/rpc-protocol.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Allocation {
    pub max_len: u32,
}

impl Allocation {
    /// Encoded size; the message's `len`.
    pub const SIZE: usize = 16;

    pub const fn to_bytes(self) -> [u8; Self::SIZE] {
        let mut b = [0u8; Self::SIZE];
        let max_len = self.max_len.to_le_bytes();
        let mut i = 0;
        while i < 4 {
            b[i] = max_len[i];
            i += 1;
        }
        b
    }

    /// Decode, rejecting a wrong length or nonzero reserved bytes.
    pub fn from_bytes(b: &[u8]) -> Option<Self> {
        let b: &[u8; Self::SIZE] = b.try_into().ok()?;
        if b[4..].iter().any(|&x| x != 0) {
            return None;
        }
        Some(Self {
            max_len: u32::from_le_bytes(b[0..4].try_into().unwrap()),
        })
    }
}

// ---------------------------------------------------------------------------
// ParseError
// ---------------------------------------------------------------------------

/// Errors from validating the transmitted header bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ParseError {
    #[error("pointer is null")]
    NullPointer,
    #[error("slice is smaller than the data its header claims")]
    SliceTooSmall,
    #[error("pointer is not {ALIGN}-byte aligned")]
    MisalignedPointer,
    #[error("payload length exceeds the buffer's capacity")]
    LengthExceedsCapacity,
    #[error("unsupported wire format / protocol version")]
    UnsupportedWireFormat,
    /// §8.1: the reserved byte (offset 7) and reserved word (offsets 24..32)
    /// are validated zero, which is what keeps them spendable on a future
    /// format change without burning a version number.
    #[error("reserved header bytes are not zero")]
    NonZeroReserved,
    #[error("unknown status code: {0}")]
    UnknownStatus(u8),
}

// ---------------------------------------------------------------------------
// PyroParser
// ---------------------------------------------------------------------------

/// Field offsets and the validators for the 48-byte header.
#[derive(Debug, Clone, Copy)]
pub struct PyroParser;

impl PyroParser {
    pub const ALIGN: usize = ALIGN;
    pub const HEADER_SIZE: usize = HEADER_SIZE;

    // Framing word.
    pub const OFFSET_LEN: usize = 0;
    pub const OFFSET_VERSION: usize = 4;
    pub const OFFSET_STATUS: usize = 5;
    pub const OFFSET_FLAGS: usize = 6;
    pub const OFFSET_RESERVED: usize = 7;
    // Schema.
    pub const OFFSET_HASH: usize = 8;
    // Tracing.
    pub const OFFSET_TRACE_ID: usize = 16;
    pub const OFFSET_PARENT_SPAN: usize = 32;
    pub const OFFSET_TIMESTAMP: usize = 40;

    /// Validate a header in a slice: size, alignment, version, reserved byte,
    /// and that the claimed payload actually fits in the slice.
    pub fn check(slice: &[u8]) -> Result<(), ParseError> {
        if slice.len() < HEADER_SIZE {
            return Err(ParseError::SliceTooSmall);
        }
        if !(slice.as_ptr() as usize).is_multiple_of(ALIGN) {
            return Err(ParseError::MisalignedPointer);
        }
        let header: &[u8; HEADER_SIZE] = slice[..HEADER_SIZE].try_into().unwrap();
        Self::check_fields(header)?;

        let len = header.payload_len() as usize;
        // The slice must hold the header *and* the payload it claims.
        if len > slice.len() - HEADER_SIZE {
            return Err(ParseError::SliceTooSmall);
        }
        Ok(())
    }

    /// Validate a header behind a raw pointer. `len` is unchecked because no
    /// capacity is known here — use [`Self::check_strict_raw`] when one is.
    ///
    /// # Safety
    /// `ptr` must be readable for at least [`HEADER_SIZE`] bytes, or null.
    pub unsafe fn check_raw(ptr: *const u8) -> Result<(), ParseError> {
        if ptr.is_null() {
            return Err(ParseError::NullPointer);
        }
        if !(ptr as usize).is_multiple_of(ALIGN) {
            return Err(ParseError::MisalignedPointer);
        }
        let header = unsafe { &*(ptr as *const [u8; HEADER_SIZE]) };
        Self::check_fields(header)
    }

    /// As [`Self::check_raw`], and additionally that the claimed payload length
    /// fits within `capacity` payload bytes.
    ///
    /// # Safety
    /// `ptr` must be readable for at least [`HEADER_SIZE`] bytes, or null.
    pub unsafe fn check_strict_raw(ptr: *const u8, capacity: usize) -> Result<(), ParseError> {
        unsafe { Self::check_raw(ptr)? };
        let header = unsafe { &*(ptr as *const [u8; HEADER_SIZE]) };
        let len = header.payload_len() as usize;
        if len > capacity {
            return Err(ParseError::LengthExceedsCapacity);
        }
        Ok(())
    }

    /// The field checks that need no bounds context: version and the reserved
    /// byte. Status is deliberately *not* rejected when unknown — a newer
    /// peer's status code must not become a parse failure.
    #[inline]
    fn check_fields(header: &[u8; HEADER_SIZE]) -> Result<(), ParseError> {
        if header[Self::OFFSET_VERSION] != PROTOCOL_VERSION {
            return Err(ParseError::UnsupportedWireFormat);
        }
        if header.reserved() != 0 {
            return Err(ParseError::NonZeroReserved);
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// The header trait stack
// ---------------------------------------------------------------------------

/// Copy the `N`-byte field at `off` out of the header. The offset is a
/// constant at every call site, so this folds to a single (aligned) load.
#[inline(always)]
fn field<const N: usize>(h: &[u8; HEADER_SIZE], off: usize) -> [u8; N] {
    h[off..off + N].try_into().unwrap()
}

#[inline(always)]
fn set_field<const N: usize>(h: &mut [u8; HEADER_SIZE], off: usize, bytes: [u8; N]) {
    h[off..off + N].copy_from_slice(&bytes);
}

/// Anything that can produce the transmitted header bytes.
pub trait HasHeader {
    fn header_bytes(&self) -> &[u8; HEADER_SIZE];
}

/// Anything that can produce them mutably.
pub trait HasHeaderMut: HasHeader {
    fn header_bytes_mut(&mut self) -> &mut [u8; HEADER_SIZE];
}

impl HasHeader for [u8; HEADER_SIZE] {
    #[inline]
    fn header_bytes(&self) -> &[u8; HEADER_SIZE] {
        self
    }
}

impl HasHeaderMut for [u8; HEADER_SIZE] {
    #[inline]
    fn header_bytes_mut(&mut self) -> &mut [u8; HEADER_SIZE] {
        self
    }
}

/// Read accessors for the §3 header. Blanket-implemented for every
/// [`HasHeader`].
pub trait PyroHeader: HasHeader {
    #[inline]
    fn payload_len(&self) -> u32 {
        u32::from_le_bytes(field(self.header_bytes(), PyroParser::OFFSET_LEN))
    }

    #[inline]
    fn version(&self) -> u8 {
        self.header_bytes()[PyroParser::OFFSET_VERSION]
    }

    #[inline]
    fn status_u8(&self) -> u8 {
        self.header_bytes()[PyroParser::OFFSET_STATUS]
    }

    #[inline]
    fn status(&self) -> Result<DataStatus, u8> {
        DataStatus::parse(self.status_u8())
    }

    #[inline]
    fn flags(&self) -> u8 {
        self.header_bytes()[PyroParser::OFFSET_FLAGS]
    }

    #[inline]
    fn reserved(&self) -> u8 {
        self.header_bytes()[PyroParser::OFFSET_RESERVED]
    }

    #[inline]
    fn schema_hash(&self) -> u64 {
        u64::from_le_bytes(field(self.header_bytes(), PyroParser::OFFSET_HASH))
    }

    #[inline]
    fn trace_id(&self) -> u128 {
        u128::from_le_bytes(field(self.header_bytes(), PyroParser::OFFSET_TRACE_ID))
    }

    #[inline]
    fn parent_span(&self) -> u64 {
        u64::from_le_bytes(field(self.header_bytes(), PyroParser::OFFSET_PARENT_SPAN))
    }

    /// Nanoseconds since the Unix epoch.
    #[inline]
    fn timestamp(&self) -> i64 {
        i64::from_le_bytes(field(self.header_bytes(), PyroParser::OFFSET_TIMESTAMP))
    }

    #[inline]
    fn is_sampled(&self) -> bool {
        self.flags() & FLAG_SAMPLED != 0
    }

    /// Answers a call. Unknown status bytes are not replies.
    #[inline]
    fn is_reply(&self) -> bool {
        self.status().is_ok_and(DataStatus::is_reply)
    }

    /// The payload is a JSON `CapturedError`. Unknown status bytes are not.
    #[inline]
    fn is_error_payload(&self) -> bool {
        self.status().is_ok_and(DataStatus::is_error)
    }

    /// The unit message: `len == 0`, `schema_hash == 0`, `status == Unit`.
    /// This triple is the one exemption from the schema-hash check (§3).
    #[inline]
    fn is_unit(&self) -> bool {
        self.payload_len() == 0
            && self.schema_hash() == 0
            && self.status_u8() == DataStatus::Unit.to_u8()
    }

    /// Compare against a locally known schema hash, honouring the unit
    /// exemption.
    #[inline]
    fn check_schema(&self, expected: u64) -> Result<(), (u64, u64)> {
        if self.is_unit() {
            return Ok(());
        }
        let got = self.schema_hash();
        if got == expected {
            Ok(())
        } else {
            Err((expected, got))
        }
    }
}

impl<T: HasHeader + ?Sized> PyroHeader for T {}

/// Write accessors for the §3 header.
pub trait PyroHeaderMut: HasHeaderMut {
    /// # Safety
    /// `len` must not exceed the buffer's payload capacity. Nothing here can
    /// see the capacity, which is exactly why this is `unsafe`.
    #[inline]
    unsafe fn set_payload_len(&mut self, len: u32) {
        set_field(
            self.header_bytes_mut(),
            PyroParser::OFFSET_LEN,
            len.to_le_bytes(),
        );
    }

    #[inline]
    fn set_version(&mut self, v: u8) {
        self.header_bytes_mut()[PyroParser::OFFSET_VERSION] = v;
    }

    #[inline]
    fn set_status(&mut self, s: DataStatus) {
        self.header_bytes_mut()[PyroParser::OFFSET_STATUS] = s.to_u8();
    }

    #[inline]
    fn set_flags(&mut self, f: u8) {
        self.header_bytes_mut()[PyroParser::OFFSET_FLAGS] = f;
    }

    #[inline]
    fn set_schema_hash(&mut self, h: u64) {
        set_field(
            self.header_bytes_mut(),
            PyroParser::OFFSET_HASH,
            h.to_le_bytes(),
        );
    }

    #[inline]
    fn set_trace_id(&mut self, trace_id: u128) {
        set_field(
            self.header_bytes_mut(),
            PyroParser::OFFSET_TRACE_ID,
            trace_id.to_le_bytes(),
        );
    }

    #[inline]
    fn set_parent_span(&mut self, span: u64) {
        set_field(
            self.header_bytes_mut(),
            PyroParser::OFFSET_PARENT_SPAN,
            span.to_le_bytes(),
        );
    }

    #[inline]
    fn set_sampled(&mut self, sampled: bool) {
        let f = &mut self.header_bytes_mut()[PyroParser::OFFSET_FLAGS];
        *f = (*f & !FLAG_SAMPLED) | if sampled { FLAG_SAMPLED } else { 0 };
    }

    /// Nanoseconds since the Unix epoch.
    #[inline]
    fn set_timestamp(&mut self, t: i64) {
        set_field(
            self.header_bytes_mut(),
            PyroParser::OFFSET_TIMESTAMP,
            t.to_le_bytes(),
        );
    }

    /// Zero the reserved byte, so the header passes
    /// [`PyroParser::check_raw`] whatever was in it before.
    #[inline]
    fn clear_reserved(&mut self) {
        self.header_bytes_mut()[PyroParser::OFFSET_RESERVED] = 0;
    }
}

impl<T: HasHeaderMut + ?Sized> PyroHeaderMut for T {}

// ---------------------------------------------------------------------------
// The unit message
// ---------------------------------------------------------------------------

/// A 16-aligned static header, so `UNIT_HEADER.as_ptr()` satisfies
/// [`PyroParser::check_raw`] with no allocation.
#[repr(C, align(16))]
pub struct StaticHeader(pub(crate) [u8; HEADER_SIZE]);

impl StaticHeader {
    #[inline]
    pub const fn as_bytes(&self) -> &[u8; HEADER_SIZE] {
        &self.0
    }
}

impl HasHeader for StaticHeader {
    #[inline]
    fn header_bytes(&self) -> &[u8; HEADER_SIZE] {
        &self.0
    }
}

/// The unit message's header: len 0, version 1, status `Unit`, no flags,
/// hash 0, no trace, timestamp 0, reserved zero.
pub const UNIT_BYTES: [u8; HEADER_SIZE] = {
    let mut h = [0u8; HEADER_SIZE];
    h[PyroParser::OFFSET_VERSION] = PROTOCOL_VERSION;
    h
};

/// The process-wide unit message. No allocation, no refcount, no pool.
pub static UNIT_HEADER: StaticHeader = StaticHeader(UNIT_BYTES);

// ---------------------------------------------------------------------------
// The schema hash — level 1 of pyro-schema's four-check compatibility ladder
// (docs/pyro-schema-crate-plan.md). Levels 2-4 (layout_eq/schema_eq/
// signature_eq) are pure functions over (inodes, names) with no transport in
// them, so they live in `pyro_schema::compat` alongside this one; re-exported
// here because `StaticHeader` const-evaluates it.
// ---------------------------------------------------------------------------

pub use pyro_schema::schema_hash;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_offsets_are_pinned() {
        assert_eq!(HEADER_SIZE, 48);
        assert_eq!(PyroParser::OFFSET_LEN, 0);
        assert_eq!(PyroParser::OFFSET_VERSION, 4);
        assert_eq!(PyroParser::OFFSET_STATUS, 5);
        assert_eq!(PyroParser::OFFSET_FLAGS, 6);
        assert_eq!(PyroParser::OFFSET_RESERVED, 7);
        assert_eq!(PyroParser::OFFSET_HASH, 8);
        assert_eq!(PyroParser::OFFSET_TRACE_ID, 16);
        assert_eq!(PyroParser::OFFSET_PARENT_SPAN, 32);
        assert_eq!(PyroParser::OFFSET_TIMESTAMP, 40);
        // No padding: the fields tile the 48 bytes.
        assert_eq!(PyroParser::OFFSET_TIMESTAMP + 8, HEADER_SIZE);
        // Every multi-byte field sits at its natural alignment.
        assert!(PyroParser::OFFSET_HASH.is_multiple_of(8));
        assert!(PyroParser::OFFSET_TRACE_ID.is_multiple_of(16));
        assert_eq!(
            PyroParser::OFFSET_TRACE_ID + 16,
            PyroParser::OFFSET_PARENT_SPAN
        );
        assert!(PyroParser::OFFSET_PARENT_SPAN.is_multiple_of(8));
        assert_eq!(
            PyroParser::OFFSET_PARENT_SPAN + 8,
            PyroParser::OFFSET_TIMESTAMP
        );
        assert!(PyroParser::OFFSET_TIMESTAMP.is_multiple_of(8));
    }

    #[test]
    fn fields_round_trip() {
        let mut h = UNIT_BYTES;
        unsafe { h.set_payload_len(0xdead_beef) };
        h.set_status(DataStatus::Reply);
        // A spare flag bit, to check set_sampled leaves it alone.
        h.set_flags(1 << 7);
        h.set_schema_hash(0x0123_4567_89ab_cdef);
        h.set_trace_id(0x0011_2233_4455_6677_fedc_ba98_7654_3210);
        h.set_parent_span(0x8899_aabb_ccdd_eeff);
        h.set_sampled(true);
        h.set_timestamp(-1_700_000_000_000_000_123);
        assert_eq!(h.payload_len(), 0xdead_beef);
        assert_eq!(h.version(), PROTOCOL_VERSION);
        assert_eq!(h.status(), Ok(DataStatus::Reply));
        assert!(h.is_reply());
        assert_eq!(h.schema_hash(), 0x0123_4567_89ab_cdef);
        assert_eq!(h.trace_id(), 0x0011_2233_4455_6677_fedc_ba98_7654_3210);
        assert_eq!(h.parent_span(), 0x8899_aabb_ccdd_eeff);
        assert!(h.is_sampled());
        assert_eq!(
            h.flags(),
            FLAG_SAMPLED | 1 << 7,
            "set_sampled must not disturb other flags"
        );
        assert_eq!(h.timestamp(), -1_700_000_000_000_000_123);
        h.set_sampled(false);
        assert!(!h.is_sampled());
        assert_eq!(h.flags(), 1 << 7);
        assert_eq!(h.reserved(), 0);
    }

    #[test]
    fn nonzero_reserved_is_rejected() {
        let mut buf = StaticHeader(UNIT_BYTES);
        assert_eq!(PyroParser::check(&buf.0), Ok(()));
        buf.0[PyroParser::OFFSET_RESERVED] = 1;
        assert_eq!(PyroParser::check(&buf.0), Err(ParseError::NonZeroReserved));
        buf.0.clear_reserved();
        assert_eq!(PyroParser::check(&buf.0), Ok(()));
    }

    #[test]
    fn full_width_trace_passes_check() {
        let mut buf = StaticHeader(UNIT_BYTES);
        buf.0.set_trace_id(u128::MAX);
        buf.0.set_parent_span(u64::MAX);
        assert_eq!(PyroParser::check(&buf.0), Ok(()));
        buf.0.clear_reserved();
        assert_eq!(buf.0.trace_id(), u128::MAX);
        assert_eq!(buf.0.parent_span(), u64::MAX);
    }

    #[test]
    fn status_round_trips() {
        for s in DataStatus::ALL {
            assert_eq!(DataStatus::parse(s.to_u8()), Ok(*s));
        }
    }

    #[test]
    fn status_classes_partition() {
        for &s in DataStatus::ALL {
            assert_ne!(s.is_ok(), s.is_error(), "{s:?}");
            // Only ok kinds carry schema data.
            assert!(!s.carries_data() || s.is_ok(), "{s:?}");
            // Every error answers a call except the connection-level one.
            assert_eq!(
                s.is_error() && s != DataStatus::Protocol,
                s.is_reply() && s != DataStatus::Reply,
                "{s:?}"
            );
            // No message goes both ways along a call.
            assert!(!(s.flows_to_callee() && s.flows_to_caller()), "{s:?}");
            // Every call message has a direction; connection messages don't.
            let connection_level = matches!(
                s,
                DataStatus::Unit
                    | DataStatus::Data
                    | DataStatus::Hello
                    | DataStatus::Close
                    | DataStatus::Allocate
                    | DataStatus::Grant
                    | DataStatus::Protocol
            );
            assert_eq!(
                s.flows_to_callee() || s.flows_to_caller(),
                !connection_level,
                "{s:?}"
            );
            assert!(!(s.carries_data() && s.carries_allocation()), "{s:?}");
        }
        assert!(DataStatus::Reply.is_reply());
        assert!(!DataStatus::Request.is_reply());
        assert!(!DataStatus::Protocol.is_reply());
        assert!(
            !DataStatus::Grant.is_reply(),
            "a grant is not the final answer"
        );
    }

    #[test]
    fn allocation_round_trips_and_rejects_reserved() {
        let a = Allocation {
            max_len: 0x0102_0304,
        };
        let b = a.to_bytes();
        assert_eq!(Allocation::from_bytes(&b), Some(a));
        for i in 4..Allocation::SIZE {
            let mut bad = b;
            bad[i] = 1;
            assert_eq!(Allocation::from_bytes(&bad), None, "reserved byte {i}");
        }
        assert_eq!(Allocation::from_bytes(&b[..15]), None);
    }

    #[test]
    fn unknown_status_is_neither_reply_nor_error() {
        let mut h = UNIT_BYTES;
        h[PyroParser::OFFSET_STATUS] = 200;
        assert!(!h.is_reply());
        assert!(!h.is_error_payload());
    }

    #[test]
    fn unknown_status_is_returned_verbatim() {
        assert_eq!(DataStatus::parse(200), Err(200));
    }

    // `schema_hash`'s own behavioral tests (never-zero, const-evaluable,
    // fixed_size/nullable_bit_index/group_count sensitivity) now live in
    // `pyro_schema::compat`'s test module, alongside the other three
    // compatibility levels — this file only re-exports the function.
}
