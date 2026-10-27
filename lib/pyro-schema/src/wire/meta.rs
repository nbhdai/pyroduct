//! [`InodeMeta`]: the message-wide settings an inode table alone doesn't
//! carry, sent as one [`INODE_SIZE`] slot at the head of the table.
//!
//! `&[Inode]` used to be enough to read a message. It no longer is: the same
//! inodes can be laid out in `u16` or `u32` [`Word`]s, and a reader has to know
//! which. `InodeMeta` holds that, and whatever message-wide setting joins it
//! later, so `(InodeMeta, &[Inode])` is what reads a message and what
//! [`schema_hash`](crate::schema_hash) fingerprints.
//!
//! # Encoding
//!
//! Exactly [`INODE_SIZE`] bytes, so the table keeps one stride:
//!
//! | byte | field | values |
//! | - | - | - |
//! | 0 | version | [`InodeMeta::VERSION`] |
//! | 1 | word | [`Word`] width in bytes: `2` (`U16`) or `4` (`U32`) |
//! | 2..12 | reserved | must be zero |
//!
//! A decoder refuses a version or reserved byte it doesn't know rather than
//! skipping it: every field here changes how the message is read.

use super::inode::INODE_SIZE;
use crate::word::Word;

/// Error returned by [`InodeMeta::decode`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum InodeMetaDecodeError {
    #[error("inode meta buffer too short")]
    TooShort,
    #[error("unsupported inode meta version: {0}")]
    UnsupportedVersion(u8),
    #[error("invalid structure word width: {0}")]
    InvalidWord(u8),
    #[error("reserved inode meta byte {0} is non-zero")]
    Reserved(usize),
}

/// Message-wide settings for an inode table. See the [module docs](self).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InodeMeta {
    /// The word the structure (topology tiers, fixed-region pointers) is
    /// written in.
    pub word: Word,
}

impl Default for InodeMeta {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl InodeMeta {
    /// Byte 0 of every encoding this build writes.
    pub const VERSION: u8 = 1;

    /// `u32` words, what a schema gets unless it asks otherwise.
    pub const DEFAULT: Self = Self::new(Word::U32);

    pub const fn new(word: Word) -> Self {
        Self { word }
    }

    pub const fn encode(&self) -> [u8; INODE_SIZE] {
        let mut buf = [0u8; INODE_SIZE];
        buf[0] = Self::VERSION;
        buf[1] = match self.word {
            Word::U16 => 2,
            Word::U32 => 4,
        };
        buf
    }

    pub const fn decode(bytes: &[u8]) -> Result<Self, InodeMetaDecodeError> {
        if bytes.len() < INODE_SIZE {
            return Err(InodeMetaDecodeError::TooShort);
        }
        if bytes[0] != Self::VERSION {
            return Err(InodeMetaDecodeError::UnsupportedVersion(bytes[0]));
        }
        let word = match bytes[1] {
            2 => Word::U16,
            4 => Word::U32,
            other => return Err(InodeMetaDecodeError::InvalidWord(other)),
        };
        let mut i = 2;
        while i < INODE_SIZE {
            if bytes[i] != 0 {
                return Err(InodeMetaDecodeError::Reserved(i));
            }
            i += 1;
        }
        Ok(Self { word })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_both_words() {
        for word in [Word::U16, Word::U32] {
            let meta = InodeMeta::new(word);
            assert_eq!(InodeMeta::decode(&meta.encode()), Ok(meta));
        }
    }

    #[test]
    fn rejects_unknown_bytes() {
        let mut bytes = InodeMeta::DEFAULT.encode();
        bytes[0] = 2;
        assert_eq!(
            InodeMeta::decode(&bytes),
            Err(InodeMetaDecodeError::UnsupportedVersion(2))
        );

        let mut bytes = InodeMeta::DEFAULT.encode();
        bytes[1] = 8;
        assert_eq!(
            InodeMeta::decode(&bytes),
            Err(InodeMetaDecodeError::InvalidWord(8))
        );

        let mut bytes = InodeMeta::DEFAULT.encode();
        bytes[11] = 1;
        assert_eq!(
            InodeMeta::decode(&bytes),
            Err(InodeMetaDecodeError::Reserved(11))
        );

        assert_eq!(
            InodeMeta::decode(&bytes[..11]),
            Err(InodeMetaDecodeError::TooShort)
        );
    }
}
