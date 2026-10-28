//! A fixed-capacity write-ahead log of pyroformat messages, on [compio]'s
//! completion-based, thread-per-core runtime.
//!
//! Every row is one sealed pyroformat message of a single schema. The file
//! carries that schema up front, the rows after it, and an index of exactly
//! `N` rows at the end:
//!
//! ```text
//! offset 0     FILE HEADER (32 bytes)
//!   0..8         magic         b"PYROWAL\0"
//!   8..10        version       u16   FORMAT_VERSION
//!   10..12       reserved      u16   must be zero
//!   12..16       capacity      u32   N, the footer's row count
//!   16..24       schema_hash   u64   schema_hash(meta, inodes); every row's header carries it
//!   24..28       schema_len    u32   byte length of the schema table below
//!   28..32       header_crc    u32   CRC-32C of bytes 0..28, then the schema table
//! offset 32    SCHEMA TABLE  SchemaParts::encode: InodeMeta + inode table, then strings
//!              zero padding to 16
//! data_start   ROWS          each 16-aligned:
//!   0..4           crc       u32   CRC-32C of the message
//!   4..16          reserved        must be zero
//!   16..           message         pyroformat header (48 bytes) + payload, 16-aligned
//!              zero padding to 16
//! footer_start FOOTER (4·N + 16 bytes), the last bytes of the file
//!   N ×          offset u32, from data_start to the row; rows past `count` are zero
//!   trailer      magic b"PYROWALF" | count u32 | footer_crc u32 (CRC-32C of everything before it)
//! ```
//!
//! [`WalWriter`] writes the header and schema first, appends rows, and writes
//! the footer on [`WalWriter::finish`]. A file without a valid footer — the
//! writer never finished — is still readable: [`WalReader::open`] then scans
//! the data region in 16-byte steps for message headers stamped with the file's
//! `schema_hash` whose message matches the CRC in front of it, skipping
//! anything else, torn rows included.
//!
//! [`WalReader`] reads rows into a pool of `K` reusable slots. Each row comes
//! back as a [`PyroView`](pyroformat::buf::PyroView) — refcounted, `Send` and
//! `Sync`, so other threads can hold it and borrow
//! [`PyroRef`](pyroformat::buf::PyroRef)s from it — and its slot returns to the
//! pool when the last handle drops, on whichever thread that happens. With all
//! `K` slots out, [`WalReader::read`] waits for one to come back.
//!
//! IO goes through compio's [`AsyncWriteAt`](compio::io::AsyncWriteAt) and
//! [`AsyncReadAt`](compio::io::AsyncReadAt). Nothing is synced to stable
//! storage; sync the file yourself where durability matters.
//!
//! [compio]: https://docs.rs/compio

mod error;
mod format;
mod message;
mod pool;
mod reader;
mod writer;

pub use error::{Result, WalError};
pub use format::{
    FILE_HEADER_SIZE, FOOTER_ENTRY_SIZE, FOOTER_MAGIC, FOOTER_TRAILER_SIZE, FORMAT_VERSION, MAGIC,
    ROW_ALIGN, ROW_PREFIX_SIZE, footer_size,
};
pub use reader::{Recovery, WalReader};
pub use writer::WalWriter;

#[cfg(test)]
mod tests;
