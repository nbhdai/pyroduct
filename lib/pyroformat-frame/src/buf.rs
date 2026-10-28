//! Owned buffers that compio's completion-based IO can borrow for the duration of an operation.

use std::mem::MaybeUninit;

use compio::buf::IoBuf;
use compio::buf::IoBufMut;
use compio::buf::SetLen;
use vortex_buffer::Alignment;
use vortex_buffer::ByteBuffer;
use vortex_buffer::ByteBufferMut;

/// A frozen [`ByteBuffer`] handed to compio for writing, without copying.
pub struct WriteBuffer(pub ByteBuffer);

impl IoBuf for WriteBuffer {
    fn as_init(&self) -> &[u8] {
        self.0.as_slice()
    }
}

/// An aligned buffer that a compio read fills with exactly `len` bytes.
///
/// compio reads up to a buffer's capacity, but an aligned [`ByteBufferMut`] may allocate more
/// capacity than requested, so the capacity exposed to compio is capped at `len`.
pub struct ReadBuffer {
    buffer: ByteBufferMut,
    len: usize,
}

impl ReadBuffer {
    /// An empty buffer that reads `len` bytes into memory aligned to `alignment`.
    pub fn new(len: usize, alignment: Alignment) -> Self {
        Self {
            buffer: ByteBufferMut::with_capacity_aligned(len, alignment),
            len,
        }
    }

    /// Whether every byte has been read.
    pub fn is_full(&self) -> bool {
        self.buffer.len() == self.len
    }

    /// The bytes read so far.
    pub fn freeze(self) -> ByteBuffer {
        self.buffer.freeze()
    }
}

impl IoBuf for ReadBuffer {
    fn as_init(&self) -> &[u8] {
        self.buffer.as_slice()
    }
}

impl SetLen for ReadBuffer {
    unsafe fn set_len(&mut self, len: usize) {
        debug_assert!(len <= self.len);
        // SAFETY: the caller guarantees the first `len` bytes are initialized, and `len` is within
        // the capacity exposed by `as_uninit`, which is within the allocation.
        unsafe { self.buffer.set_len(len) }
    }
}

impl IoBufMut for ReadBuffer {
    fn as_uninit(&mut self) -> &mut [MaybeUninit<u8>] {
        let initialized = self.buffer.len();
        let len = self.len;
        let ptr = self.buffer.as_mut_ptr().cast::<MaybeUninit<u8>>();
        // SAFETY: the allocation holds at least `len` bytes, since it was created with that
        // capacity, and the slice is uniquely borrowed through `&mut self`. The first
        // `initialized` bytes are initialized and the rest may be written by the read.
        debug_assert!(initialized <= len);
        unsafe { std::slice::from_raw_parts_mut(ptr, len) }
    }
}

#[cfg(test)]
mod tests {
    use compio::buf::IoBufMut;
    use compio::buf::SetLen;
    use vortex_buffer::Alignment;

    use super::ReadBuffer;

    #[test]
    fn read_buffer_exposes_exactly_len() {
        let mut buffer = ReadBuffer::new(10, Alignment::new(64));
        assert_eq!(buffer.as_uninit().len(), 10);
        for byte in buffer.as_uninit() {
            byte.write(7);
        }
        // SAFETY: every byte was written above.
        unsafe { buffer.set_len(10) };
        assert!(buffer.is_full());
        let frozen = buffer.freeze();
        assert_eq!(frozen.as_slice(), &[7; 10]);
        assert!(frozen.is_aligned(Alignment::new(64)));
    }
}
