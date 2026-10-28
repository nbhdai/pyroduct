use std::io;

/// Everything that can go wrong writing or reading a WAL.
#[derive(Debug, thiserror::Error)]
pub enum WalError {
    #[error(transparent)]
    Io(#[from] io::Error),
    /// The file does not start with a valid WAL header and schema table.
    #[error("not a pyro WAL: {0}")]
    BadHeader(&'static str),
    /// The schema table does not decode, or does not hash to `schema_hash`.
    #[error("bad WAL schema: {0}")]
    BadSchema(&'static str),
    /// Every one of the footer's rows is taken.
    #[error("WAL is full: all {capacity} rows written")]
    Full { capacity: u32 },
    /// The row would start past what a `u32` footer offset can name.
    #[error("WAL data region is too large for u32 row offsets")]
    TooLarge,
    /// An appended message is not a data message of this WAL's schema.
    #[error("message rejected: {0}")]
    BadMessage(&'static str),
    #[error("row {row} out of range: the WAL has {len} rows")]
    OutOfRange { row: usize, len: usize },
    /// A row's bytes do not match its footer entry.
    #[error("row {row} is corrupt: {reason}")]
    Corrupt { row: usize, reason: &'static str },
}

pub type Result<T, E = WalError> = std::result::Result<T, E>;
