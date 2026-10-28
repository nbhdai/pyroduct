//! IO traits for compio's thread-per-core runtime.
//!
//! Unlike `vortex_io::VortexReadAt` and `vortex_io::VortexWrite`, implementations and their
//! futures need not be `Send`: they run on the compio thread that created them and pass owned
//! buffers to the kernel, as completion-based IO requires.

use std::future::Future;
use std::io;

use compio::BufResult;
use compio::buf::IoBuf;
use compio::buf::IoBufMut;
use compio::fs::File;
use compio::io::AsyncReadAtExt;
use compio::io::AsyncWriteAtExt;
use vortex_io::CoalesceConfig;

/// Default number of reads [`CompioReadAt`] sources keep in flight.
pub const DEFAULT_READ_CONCURRENCY: usize = 32;

/// Default number of writes [`CompioWriteAt`] sinks keep in flight.
pub const DEFAULT_WRITE_CONCURRENCY: usize = 4;

/// A positional byte source on a compio runtime.
pub trait CompioReadAt {
    /// The total length of the source in bytes.
    fn size(&self) -> impl Future<Output = io::Result<u64>>;

    /// Fills `buf` to its capacity with the bytes starting at `offset`, failing with
    /// [`UnexpectedEof`][io::ErrorKind::UnexpectedEof] if the source ends first.
    fn read_exact_at<B: IoBufMut>(
        &self,
        buf: B,
        offset: u64,
    ) -> impl Future<Output = BufResult<(), B>>;

    /// How nearby reads are merged into fewer, larger ones, or `None` to read every segment
    /// separately.
    fn coalesce_config(&self) -> Option<CoalesceConfig> {
        Some(CoalesceConfig::file())
    }

    /// The number of reads to keep in flight at once.
    fn concurrency(&self) -> usize {
        DEFAULT_READ_CONCURRENCY
    }
}

/// A positional byte sink on a compio runtime that accepts several writes in flight at once.
///
/// Sinks without positions, such as sockets, are written through `compio::io::AsyncWrite`
/// instead; see [`crate::write`].
pub trait CompioWriteAt {
    /// Writes all of `buf` at `offset`. Concurrent calls always name disjoint ranges.
    fn write_all_at<B: IoBuf>(&self, buf: B, offset: u64)
    -> impl Future<Output = BufResult<(), B>>;

    /// The number of writes to keep in flight at once.
    fn max_in_flight(&self) -> usize {
        DEFAULT_WRITE_CONCURRENCY
    }
}

impl CompioReadAt for File {
    async fn size(&self) -> io::Result<u64> {
        Ok(self.metadata().await?.len())
    }

    async fn read_exact_at<B: IoBufMut>(&self, buf: B, offset: u64) -> BufResult<(), B> {
        AsyncReadAtExt::read_exact_at(self, buf, offset).await
    }
}

impl CompioWriteAt for File {
    async fn write_all_at<B: IoBuf>(&self, buf: B, offset: u64) -> BufResult<(), B> {
        // Positional writes through a shared reference let several run at once.
        let mut file = self;
        AsyncWriteAtExt::write_all_at(&mut file, buf, offset).await
    }
}

impl<T: CompioReadAt + ?Sized> CompioReadAt for &T {
    fn size(&self) -> impl Future<Output = io::Result<u64>> {
        (**self).size()
    }

    fn read_exact_at<B: IoBufMut>(
        &self,
        buf: B,
        offset: u64,
    ) -> impl Future<Output = BufResult<(), B>> {
        (**self).read_exact_at(buf, offset)
    }

    fn coalesce_config(&self) -> Option<CoalesceConfig> {
        (**self).coalesce_config()
    }

    fn concurrency(&self) -> usize {
        (**self).concurrency()
    }
}

impl<T: CompioWriteAt + ?Sized> CompioWriteAt for &T {
    fn write_all_at<B: IoBuf>(
        &self,
        buf: B,
        offset: u64,
    ) -> impl Future<Output = BufResult<(), B>> {
        (**self).write_all_at(buf, offset)
    }

    fn max_in_flight(&self) -> usize {
        (**self).max_in_flight()
    }
}
