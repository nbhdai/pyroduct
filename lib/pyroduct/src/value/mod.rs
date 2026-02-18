use core::fmt;

use thiserror::Error;
mod get_value;
mod time;
mod typeable;
mod value;
pub use value::*;
pub mod coerce;
mod repair;
// mod bow;
mod bridgeable;
mod schema;
pub use repair::ScalarRepairError;
pub use schema::{PrimitiveDataType, PyroField, PyroSchema, PyroType, ValueSchemaInferenceError};
pub use typeable::{Typeable, TypeableRow};

#[cfg(not(target_arch = "wasm32"))]
pub mod arrow;

pub mod deep_ref;
pub use deep_ref::DeepRef;
pub use time::{ArchivedTime, Time};

pub trait ToRow {
    /// Convert a borrowed reference to a PyroRow with borrowed data.
    /// Field names are static strings from compile time.
    fn to_row(&self) -> PyroRow<'_>;
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct InvalidValue {
    pub value: Option<PyroValue<'static>>,
    pub v_type: PyroType<'static>,
}

impl InvalidValue {
    /// Creates a new InvalidValue, preserving the data by converting it to 'static.
    /// Use this for small scalar values that are cheap to clone and store.
    pub fn new(value: &PyroValue<'_>) -> Self {
        Self {
            v_type: PyroType::from_value(value).into_owned(),
            value: Some(value.to_static()),
        }
    }

    /// Creates a new InvalidValue WITHOUT the data (storing only the type).
    /// Use this for large structures (Groups, huge Lists, Strings) where storing
    /// the invalid data would be too expensive or redundant.
    pub fn new_large(value: &PyroValue<'_>) -> Self {
        Self {
            v_type: PyroType::from_value(value).into_owned(),
            value: None,
        }
    }
}

impl fmt::Display for InvalidValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.value {
            // Case 1: Value was preserved (small scalar)
            Some(v) => write!(f, "Invalid(Type: {}, Value: {})", self.v_type, v),

            // Case 2: Value was omitted (large structure)
            None => write!(f, "Invalid(Type: {})", self.v_type),
        }
    }
}

#[derive(Debug, Error)]
pub enum ValueError {
    #[error("Invalid Time")]
    Time,
    #[error("Invalid Scalar")]
    InvalidScalar(Box<InvalidValue>),
    #[error("Can't cast {0:?} to {1}")]
    Cast(Box<InvalidValue>, PyroType<'static>),

    #[cfg(not(target_arch = "wasm32"))]
    #[error("Arrow Error")]
    ArrowError(#[from] ::arrow::error::ArrowError),

    #[error("Method `{0}` is not available for type `{1}`")]
    Unimplemented(String, String),
}

impl ValueError {
    pub fn invalid<'a>(value: impl Into<PyroValue<'a>>) -> Self {
        ValueError::InvalidScalar(InvalidValue::new(&value.into()).into())
    }
    pub fn invalid_large<'a>(value: impl Into<PyroValue<'a>>) -> Self {
        ValueError::InvalidScalar(InvalidValue::new_large(&value.into()).into())
    }
    pub fn cast<'a>(value: impl Into<PyroValue<'a>>, target: &PyroType<'_>) -> Self {
        ValueError::Cast(
            InvalidValue::new(&value.into()).into(),
            target.clone().into_owned(),
        )
    }
    pub fn cast_large<'a>(value: impl Into<PyroValue<'a>>, target: &PyroType<'a>) -> Self {
        ValueError::Cast(
            InvalidValue::new_large(&value.into()).into(),
            target.clone().into_owned(),
        )
    }
}

type ValueResult<A> = std::result::Result<A, ValueError>;

impl PartialEq for ValueError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Unimplemented(l0, l1), Self::Unimplemented(r0, r1)) => l0 == r0 && l1 == r1,
            (Self::InvalidScalar(l0), Self::InvalidScalar(r0)) => l0 == r0,
            // (Self::ArrowError(_), Self::ArrowError(_)) => true,
            (Self::Cast(l0, l1), Self::Cast(r0, r1)) => l0 == r0 && l1 == r1,
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}
