//! The on-disk layout: file header, footer, and the row-header probe the
//! recovery scan uses. Pure byte functions, no IO. See the crate docs for the
//! picture.

use pyroformat::buf::{ALIGN, DataStatus, HEADER_SIZE, PROTOCOL_VERSION, PyroHeader, PyroParser};

use crate::error::{Result, WalError};

/// First eight bytes of every WAL file.
pub const MAGIC: [u8; 8] = *b"PYROWAL\0";
/// Opens the footer trailer, the last 16 bytes of a finished WAL.
pub const FOOTER_MAGIC: [u8; 8] = *b"PYROWALF";
/// The layout version at header offset 8.
pub const FORMAT_VERSION: u16 = 1;

/// The fixed part of the file header, ahead of the schema table.
pub const FILE_HEADER_SIZE: usize = 32;
/// One footer row: its `offset u32`.
pub const FOOTER_ENTRY_SIZE: usize = 4;
/// Ahead of every row's message: `crc u32 | reserved [u8; 12]`, the CRC-32C of
/// the message. Sized so the message stays 16-aligned.
pub const ROW_PREFIX_SIZE: usize = 16;
/// The footer trailer: `magic [u8; 8] | count u32 | footer_crc u32`.
pub const FOOTER_TRAILER_SIZE: usize = 16;
/// Every row, and the footer, starts on this boundary; it is the pyroformat
/// buffer alignment, so a row read to an aligned slot is a valid message.
pub const ROW_ALIGN: u64 = ALIGN as u64;

const OFF_VERSION: usize = 8;
const OFF_RESERVED: usize = 10;
const OFF_CAPACITY: usize = 12;
const OFF_HASH: usize = 16;
const OFF_SCHEMA_LEN: usize = 24;
const OFF_HEADER_CRC: usize = 28;

/// Total footer bytes for a WAL of `capacity` rows.
pub const fn footer_size(capacity: u32) -> u64 {
    capacity as u64 * FOOTER_ENTRY_SIZE as u64 + FOOTER_TRAILER_SIZE as u64
}

pub(crate) const fn align_up(pos: u64) -> u64 {
    (pos + ROW_ALIGN - 1) & !(ROW_ALIGN - 1)
}

pub(crate) fn crc32c(bytes: &[u8]) -> u32 {
    crc_fast::checksum(crc_fast::CrcAlgorithm::Crc32Iscsi, bytes) as u32
}

fn crc32c_parts(parts: &[&[u8]]) -> u32 {
    let mut digest = crc_fast::Digest::new(crc_fast::CrcAlgorithm::Crc32Iscsi);
    for part in parts {
        digest.update(part);
    }
    digest.finalize() as u32
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().unwrap())
}

/// The fixed file header, decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FileHeader {
    pub capacity: u32,
    pub schema_hash: u64,
    pub schema_len: u32,
    pub crc: u32,
}

impl FileHeader {
    /// Where the rows begin: after the schema table, rounded up to 16.
    pub fn data_start(&self) -> u64 {
        align_up(FILE_HEADER_SIZE as u64 + self.schema_len as u64)
    }

    /// The fixed header with the schema table behind it, zero-padded to
    /// [`Self::data_start`]: everything a writer puts down before its first row.
    pub fn encode(capacity: u32, schema_hash: u64, schema: &[u8]) -> Result<Vec<u8>> {
        let schema_len: u32 = schema
            .len()
            .try_into()
            .map_err(|_| WalError::BadSchema("schema table exceeds u32::MAX bytes"))?;
        let header = FileHeader {
            capacity,
            schema_hash,
            schema_len,
            crc: 0,
        };
        let mut out = Vec::with_capacity(header.data_start() as usize);
        out.extend_from_slice(&MAGIC);
        out.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&capacity.to_le_bytes());
        out.extend_from_slice(&schema_hash.to_le_bytes());
        out.extend_from_slice(&schema_len.to_le_bytes());
        let crc = crc32c_parts(&[&out, schema]);
        out.extend_from_slice(&crc.to_le_bytes());
        debug_assert_eq!(out.len(), FILE_HEADER_SIZE);
        out.extend_from_slice(schema);
        out.resize(header.data_start() as usize, 0);
        Ok(out)
    }

    /// Decode the fixed header. The CRC also covers the schema table, so it is
    /// checked separately by [`Self::check_crc`] once that has been read.
    pub fn decode(b: &[u8; FILE_HEADER_SIZE]) -> Result<Self> {
        if b[..8] != MAGIC {
            return Err(WalError::BadHeader("bad magic"));
        }
        if u16::from_le_bytes([b[OFF_VERSION], b[OFF_VERSION + 1]]) != FORMAT_VERSION {
            return Err(WalError::BadHeader("unsupported format version"));
        }
        if b[OFF_RESERVED..OFF_RESERVED + 2] != [0, 0] {
            return Err(WalError::BadHeader("nonzero reserved bytes"));
        }
        Ok(Self {
            capacity: u32_at(b, OFF_CAPACITY),
            schema_hash: u64::from_le_bytes(b[OFF_HASH..OFF_HASH + 8].try_into().unwrap()),
            schema_len: u32_at(b, OFF_SCHEMA_LEN),
            crc: u32_at(b, OFF_HEADER_CRC),
        })
    }

    pub fn check_crc(&self, fixed: &[u8; FILE_HEADER_SIZE], schema: &[u8]) -> Result<()> {
        if crc32c_parts(&[&fixed[..OFF_HEADER_CRC], schema]) != self.crc {
            return Err(WalError::BadHeader("header CRC mismatch"));
        }
        Ok(())
    }
}

/// A row's prefix, for a message whose CRC-32C is `crc`.
pub(crate) fn encode_row_prefix(crc: u32) -> [u8; ROW_PREFIX_SIZE] {
    let mut prefix = [0; ROW_PREFIX_SIZE];
    prefix[..4].copy_from_slice(&crc.to_le_bytes());
    prefix
}

/// The CRC a row prefix records, or `None` if its reserved bytes are not zero.
pub(crate) fn decode_row_prefix(prefix: &[u8]) -> Option<u32> {
    let prefix = prefix.get(..ROW_PREFIX_SIZE)?;
    prefix[4..]
        .iter()
        .all(|&b| b == 0)
        .then(|| u32_at(prefix, 0))
}

/// The whole footer: `capacity` row offsets from `data_start` (unused ones
/// zero), then the trailer.
pub(crate) fn encode_footer(capacity: u32, rows: &[u32]) -> Vec<u8> {
    debug_assert!(rows.len() <= capacity as usize);
    let mut out = Vec::with_capacity(footer_size(capacity) as usize);
    for offset in rows {
        out.extend_from_slice(&offset.to_le_bytes());
    }
    out.resize(capacity as usize * FOOTER_ENTRY_SIZE, 0);
    out.extend_from_slice(&FOOTER_MAGIC);
    out.extend_from_slice(&(rows.len() as u32).to_le_bytes());
    let crc = crc32c(&out);
    out.extend_from_slice(&crc.to_le_bytes());
    out
}

/// Decode a footer read from the last [`footer_size`] bytes. `None` when it is
/// not a footer — missing, torn, or describing rows that cannot be there —
/// which sends the reader to recovery.
pub(crate) fn decode_footer(capacity: u32, footer: &[u8], data_len: u64) -> Option<Vec<u32>> {
    if footer.len() as u64 != footer_size(capacity) {
        return None;
    }
    let trailer = footer.len() - FOOTER_TRAILER_SIZE;
    if footer[trailer..trailer + 8] != FOOTER_MAGIC {
        return None;
    }
    let count = u32_at(footer, trailer + 8);
    if crc32c(&footer[..trailer + 12]) != u32_at(footer, trailer + 12) || count > capacity {
        return None;
    }
    let mut rows = Vec::with_capacity(count as usize);
    for i in 0..count as usize {
        let offset = u32_at(footer, i * FOOTER_ENTRY_SIZE);
        let after_last = rows.last().is_none_or(|&prev| offset > prev);
        // Each row holds at least its prefix and a message header.
        let min_end = offset as u64 + (ROW_PREFIX_SIZE + HEADER_SIZE) as u64;
        if !(offset as u64).is_multiple_of(ROW_ALIGN) || !after_last || min_end > data_len {
            return None;
        }
        rows.push(offset);
    }
    Some(rows)
}

/// The header checks a row must pass to be this WAL's data: the pyroformat
/// field checks, `status == Data`, and the file's schema hash. Returns the
/// payload length.
pub(crate) fn probe_row_header(header: &[u8; HEADER_SIZE], schema_hash: u64) -> Option<u32> {
    let fields_ok = header[PyroParser::OFFSET_VERSION] == PROTOCOL_VERSION
        && header[PyroParser::OFFSET_RESERVED] == 0;
    let is_data = header[PyroParser::OFFSET_STATUS] == DataStatus::Data.to_u8();
    (fields_ok && is_data && header.schema_hash() == schema_hash).then(|| header.payload_len())
}
