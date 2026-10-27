//! [`PyroSink`] — one write target for the wire-format writers.
//!
//! [`PyroVec`] is self-owned, growable, host-heap. A pooled chunk or a
//! registered RDMA slab slot is a [`PyroBuf`] instead — fixed capacity,
//! refcounted. Before this trait, `MessageWriter::write` and every field
//! writer beneath it took `&mut PyroVec` concretely, so the encoder could
//! only fill the one buffer kind the RPC layer never uses. See
//! [docs/write-sink-plan.md](../../../docs/write-sink-plan.md).
//!
//! Every mutating method is fallible: [`PyroVec`] can only fail on
//! allocation, [`PyroBuf`] fails with [`WireError::CapacityExceeded`] once
//! its fixed capacity is exhausted. A [`PyroBuf`] is exclusive by
//! construction, so neither checks for sharing.

use crate::buf::PyroBuf;
use crate::errors::WireError;

/// A write target the wire-format writers can fill: [`PyroVec`] or
/// [`PyroBuf`].
///
/// `reserve` deliberately is not here: a fixed-capacity sink cannot honour
/// it.
pub trait PyroSink {
    /// prepare for the variable sized portion of a message
    fn reallocate(
        &mut self,
        capacity: usize,
    ) -> impl std::future::Future<Output = Result<(), WireError>> + Send;

    /// prepare for the variable sized portion of a message
    fn reallocate_sync(&mut self, capacity: usize) -> Result<(), WireError>;

    /// Cuts the variable portion to `variable` bytes, keeping what is
    /// written before it. Never grows: past the current length it does
    /// nothing, as [`Vec::shrink_to`].
    fn shrink_to(&mut self, variable: usize);
    fn schema_hash(&self) -> u64;
    fn fixed_len(&self) -> usize;

    fn len(&self) -> usize;
    fn capacity(&self) -> usize;
    #[inline]
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The written region, for the `copy_from_slice` / back-patch sites.
    /// Never grows; use the methods above for that.
    fn payload(&self) -> &[u8];
    fn payload_mut(&mut self) -> &mut [u8];
}

/// A message being written in local memory.
///
/// Only the fixed region starts zeroed: its layout assumes it (a null scalar
/// leaves its slot as the layout zeroed it, and the bitmap's spare bits are
/// never written). The variable region is left as the allocator hands it
/// over, and every writer writes each byte of it itself, alignment padding
/// included, so the message is fully written by the end.
pub struct LocalMemSink {
    pub schema_hash: u64,
    pub buf: PyroBuf,
    pub fixed: usize,
    pub variable: usize,
}

impl LocalMemSink {
    #[inline]
    pub fn new(fixed: usize, schema_hash: u64) -> Self {
        Self::with_variable(fixed, schema_hash, 0)
    }

    #[inline]
    pub fn with_variable(fixed: usize, schema_hash: u64, variable: usize) -> Self {
        Self {
            schema_hash,
            buf: fresh(fixed, variable, schema_hash),
            fixed,
            variable,
        }
    }
}

/// A new buffer of `fixed + variable` bytes: the fixed region zeroed, the
/// variable region as allocated.
#[inline(always)]
fn fresh(fixed: usize, variable: usize, schema_hash: u64) -> PyroBuf {
    let len = fixed + variable;
    let mut buf = PyroBuf::new_valid_uninit(len, schema_hash);
    buf.capacity_mut()[..fixed].fill(0);
    unsafe { buf.set_len_unchecked(len) };
    buf
}

impl PyroSink for LocalMemSink {
    async fn reallocate(&mut self, variable: usize) -> Result<(), WireError> {
        self.reallocate_sync(variable)
    }

    /// Sizes the message to exactly `fixed + variable`. The allocation can
    /// be larger (it may start at the type's high-water mark); then it is
    /// kept and only the length changes, so the buffer handed back is the
    /// message and nothing past it.
    ///
    /// Growing past the allocation starts a fresh buffer and carries nothing
    /// over, so it is only called before anything is written.
    #[inline]
    fn reallocate_sync(&mut self, variable: usize) -> Result<(), WireError> {
        let capacity = self.fixed + variable;
        if capacity > self.buf.capacity() {
            self.buf = fresh(self.fixed, variable, self.schema_hash);
        }
        // Within capacity: checked, or just allocated.
        unsafe { self.buf.set_len_unchecked(capacity) };
        self.variable = variable;
        Ok(())
    }

    #[inline]
    fn shrink_to(&mut self, variable: usize) {
        let variable = variable.min(self.variable);
        // At most the current length, which is within capacity.
        unsafe { self.buf.set_len_unchecked(self.fixed + variable) };
        self.variable = variable;
    }

    #[inline]
    fn fixed_len(&self) -> usize {
        self.fixed
    }

    #[inline]
    fn schema_hash(&self) -> u64 {
        self.schema_hash
    }

    #[inline]
    fn len(&self) -> usize {
        self.buf.len()
    }
    #[inline]
    fn capacity(&self) -> usize {
        self.buf.capacity()
    }

    #[inline]
    fn payload(&self) -> &[u8] {
        &self.buf.payload()
    }
    #[inline]
    fn payload_mut(&mut self) -> &mut [u8] {
        &mut self.buf.capacity_mut()[..self.fixed + self.variable]
    }
}

impl std::ops::Deref for LocalMemSink {
    type Target = [u8];

    /// The message written so far: the fixed region plus the variable
    /// region last allocated.
    #[inline]
    fn deref(&self) -> &[u8] {
        &self.buf.payload()[..self.fixed + self.variable]
    }
}
