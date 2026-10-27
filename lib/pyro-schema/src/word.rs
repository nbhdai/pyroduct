//! The word the structure is written in: topology tiers and the fixed
//! region's pointers (`(start, count)` slots). Chosen per message at compile
//! time, as the `W` parameter of [`Writer`](super::Writer).

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Word {
    U16,
    U32,
}

/// An unsigned little-endian word the structure can be written in.
///
/// Everything that goes through it is handled as a `usize` byte offset or
/// count, so callers never see which width is in use.
pub trait StructureWord:
    Copy + PartialOrd + Ord + Eq + Default + std::fmt::Debug + 'static
{
    /// Bytes per word, and the alignment topology tiers start on.
    const SIZE: usize;

    /// Opens every topology tier, standing in for its implicit leading `0`.
    const RESET: Self;

    const WORD: Word;

    /// A plain cast: a value that doesn't fit wraps. Nothing here reports an
    /// overflow — the raw lengths behind every count also go into the plan's
    /// `byte_offset`, which the caller checks against [`Self::RESET`].
    fn from_usize(value: usize) -> Self;

    fn to_usize(self) -> usize;

    fn write(self, buffer: &mut [u8], at: usize);

    fn read(buffer: &[u8], at: usize) -> Self;

    /// Adds `by` as a plain cast does: the write passes only add to offsets
    /// the plan has already checked fit under [`Self::RESET`].
    #[inline]
    fn add_usize(&mut self, by: usize) {
        *self = Self::from_usize(self.to_usize() + by);
    }

    /// [`Self::add_usize`] for the planning pass: stops at `RESET` instead of
    /// wrapping past it, so a count too big for the word stays visible as
    /// `RESET` for the caller to reject.
    #[inline]
    fn add_planned(&mut self, by: usize) {
        let reset = Self::RESET.to_usize();
        *self = Self::from_usize(self.to_usize().saturating_add(by).min(reset));
    }
}

impl StructureWord for u32 {
    const SIZE: usize = std::mem::size_of::<u32>();
    const RESET: Self = u32::MAX;
    const WORD: Word = Word::U32;

    #[inline]
    fn from_usize(value: usize) -> Self {
        value as u32
    }

    #[inline]
    fn to_usize(self) -> usize {
        self as usize
    }

    #[inline]
    fn write(self, buffer: &mut [u8], at: usize) {
        buffer[at..at + Self::SIZE].copy_from_slice(&self.to_le_bytes());
    }

    #[inline]
    fn read(buffer: &[u8], at: usize) -> Self {
        u32::from_le_bytes(buffer[at..at + Self::SIZE].try_into().unwrap())
    }
}

impl StructureWord for u16 {
    const SIZE: usize = std::mem::size_of::<u16>();
    const RESET: Self = u16::MAX;
    const WORD: Word = Word::U16;

    #[inline]
    fn from_usize(value: usize) -> Self {
        value as u16
    }

    #[inline]
    fn to_usize(self) -> usize {
        self as usize
    }

    #[inline]
    fn write(self, buffer: &mut [u8], at: usize) {
        buffer[at..at + Self::SIZE].copy_from_slice(&self.to_le_bytes());
    }

    #[inline]
    fn read(buffer: &[u8], at: usize) -> Self {
        u16::from_le_bytes(buffer[at..at + Self::SIZE].try_into().unwrap())
    }
}

/// Writes `value` as one `W` word at `at`.
#[inline]
pub fn write_word<W: StructureWord>(buffer: &mut [u8], at: usize, value: usize) {
    W::from_usize(value).write(buffer, at);
}
