use arrow_schema::{ArrowError, DataType};
use thiserror::Error;
mod arrow_value;
pub use arrow_value::*;

mod repair;
pub use repair::ScalarRepairError;

#[cfg(not(target_arch = "wasm32"))]
mod record_batch;
#[cfg(not(target_arch = "wasm32"))]
pub use record_batch::*;
#[cfg(not(target_arch = "wasm32"))]
mod scalar;
#[cfg(not(target_arch = "wasm32"))]
pub use scalar::*;

pub mod deep_ref;
pub use deep_ref::DeepRef;
pub mod from_row;
pub use from_row::{FromRow, FromValue};
pub mod to_row;
pub use to_row::{ToRow, ToValue};

/// Derives a "Ref" struct (a view struct) and implements the `FromRow` trait.
///
/// Example:
/// ```rust
/// use arrow_scalars::{FromRow, DeepRef};
///
/// #[derive(FromRow, DeepRef)]
/// struct Foo { val: String }
/// // Generates:
/// // struct FooRef<'a> { val: &'a str }
/// // impl<'a> FromRow<'a> for FooRef<'a> { ... }
/// ```
#[cfg(feature = "macros")]
pub use arrow_derive::FromRow;

/// Derives the `AsDeepRef` trait for both the original struct AND its rkyv `Archived` counterpart.
/// This requires that the "Ref" struct (e.g., FooRef) already exists (usually via `ArrowRef`).
///
/// Example:
/// ```rust
/// use arrow_scalars::DeepRef;
///
/// #[derive(DeepRef)]
/// struct Foo { val: String }
/// // Generates:
/// // impl AsDeepRef for Foo { type Ref<'a> = FooRef<'a>; ... }
/// ```
#[cfg(feature = "macros")]
pub use arrow_derive::DeepRef;

/// Derives the `ToRow` trait for converting structs into ArrowRow/ArrowValue.
/// This is the opposite of `ArrowRef` which extracts references from ArrowRow.
///
/// Example:
/// ```rust
/// use arrow_scalars::ToRow;
///
/// #[derive(ToRow)]
/// struct Foo { val: String }
/// // Generates:
/// // impl ToRow for Foo {
/// //     fn to_arrow_row(&self) -> ArrowRow<'_> { ... }
/// //     fn to_arrow_row_owned(self) -> ArrowRow<'static> { ... }
/// //     fn to_arrow_value(&self) -> ArrowValue<'_> { ... }
/// //     fn to_arrow_value_owned(self) -> ArrowValue<'static> { ... }
/// // }
/// ```
#[cfg(feature = "macros")]
pub use arrow_derive::ToRow;

#[derive(Debug, Error)]
pub enum ArrowScalarError {
    #[error("Method `{0}` is not available for type `{1}`")]
    Unimplemented(String, String),
    #[error("Invalid Scalar")]
    InvalidScalar(ArrowValue<'static>),
    #[error("Out of Bounds Access Error")]
    AccessError,
    #[error("Arrow Error")]
    ArrowError(#[from] ArrowError),
    #[error("Value out of range")]
    NumberOutOfRange,
    #[error("Can't cast {0:?} to {1}")]
    Cast(ArrowValue<'static>, DataType),
}

type Result<A> = std::result::Result<A, ArrowScalarError>;

impl PartialEq for ArrowScalarError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Unimplemented(l0, l1), Self::Unimplemented(r0, r1)) => l0 == r0 && l1 == r1,
            (Self::InvalidScalar(l0), Self::InvalidScalar(r0)) => l0 == r0,
            (Self::ArrowError(_), Self::ArrowError(_)) => true,
            (Self::Cast(l0, l1), Self::Cast(r0, r1)) => l0 == r0 && l1 == r1,
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}
