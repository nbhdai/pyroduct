use std::fmt;
use std::ops::Deref;

use crate::format::{TypedWrapper, Wrapper};

// ─── Owned buffer ────────────────────────────────────────────────────────────

/// A type-safe wrapper around a `PyroVec` whose JSON payload has already been
/// deserialized into `T`.
///
/// Unlike `TypedBuf` (rkyv) there is no archived reference — the value is owned.
pub struct JsonBuf<B, T> {
    pub(super) vec: B,
    pub(super) value: T,
}

impl<B, T: fmt::Debug> fmt::Debug for JsonBuf<B, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.value.fmt(f)
    }
}

impl<B, T> Wrapper for JsonBuf<B, T> {
    type Wrapping = B;
    fn data(&self) -> &Self::Wrapping {
        &self.vec
    }
    fn into_data(self) -> Self::Wrapping {
        self.vec
    }
}

/// For JSON the "parsed type" exposed through `Deref` is just `T` itself.
impl<B, T> TypedWrapper<T> for JsonBuf<B, T> {}

impl<B, T> Deref for JsonBuf<B, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}
