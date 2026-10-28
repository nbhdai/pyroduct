//! Read and write Vortex files on [compio]'s completion-based, thread-per-core runtime.
//!
//! IO goes through the traits in this crate, whose futures need not be `Send`: each runs on the
//! compio thread that issued it and hands owned buffers to the kernel. Compression and decoding
//! stay on a separate CPU runtime, such as a [`CpuRuntime`], so they never block IO.
//!
//! - [`EncodeOptions::encode`] turns an array stream into a [`FileEncoder`] that yields the file's
//!   bytes in order without doing IO. [`write`] and [`write_at`] write each buffer as it is
//!   produced; `write_at` keeps several positional writes in flight.
//! - [`CompioOpenOptions::open`] reads the footer, and [`CompioFile::read`] fetches every segment
//!   a [`Projection`] needs in one burst of coalesced reads, returning a `vortex_file::VortexFile`
//!   whose scans resolve segments from memory.
//!
//! Only the public `vortex-*` APIs are used. The encoder is adapted from `vortex-file` 0.88.0,
//! Copyright the Vortex contributors, Apache-2.0.
//!
//! [compio]: https://docs.rs/compio

mod buf;
mod encode;
mod read;
mod runtime;
mod traits;
mod write;

pub use buf::*;
pub use encode::*;
pub use read::*;
pub use runtime::*;
pub use traits::*;
pub use write::*;

#[cfg(test)]
mod tests;
