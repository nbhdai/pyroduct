//! Bridgeable implementations for PyroRow and types convertible to/from PyroRow.
//!
//! This module provides:
//! 1. `Bridgeable` impl for `PyroRow<'static>` (aka `PyroRow`) via rkyv.
//! 2. A `RowBridgeable` trait + blanket impl for types `T` that implement
//!    `TryFrom<PyroRow<'static>>` and `Into<PyroRow<'static>>`, allowing them
//!    to be shipped/exposed through PyroRow's rkyv serialization.
//!
//! # Usage
//!
//! ```rust,ignore
//! use pyro_vec::value::{PyroRow, PyroValue};
//! use pyro_vec::Bridgeable;
//!
//! // PyroRow is directly Bridgeable:
//! let row = PyroRow::from([
//!     ("id", PyroValue::from(42i32)),
//!     ("name", PyroValue::from("hello")),
//! ]).into_owned();
//!
//! let vec = row.to_wire().unwrap();
//! ```

use rkyv::rancor::Error as RancorError;

use crate::{
    CapturedError, PyroError,
    format::{
        PyroRef, PyroVec, value::{ArchivedPyroRow, ArchivedPyroValue}
    },
};

use super::{PyroRow, PyroValue};

impl<'a> PyroRow<'a> {
    /// Converts this into an rkyv formatted pyrovec.
    pub fn to_wire(&self) -> Result<PyroVec, PyroError> {
        let bytes = rkyv::to_bytes::<RancorError>(self)
            .map_err(|e| PyroError::serialization(CapturedError::new(e)))?;
        let mut vec = PyroVec::with_capacity(bytes.len());
        vec.extend_from_slice(&bytes);
        Ok(vec)
    }

    ///
    pub fn parse_wire(view: &'a PyroRef<'a>) -> Result<Self, PyroError> {
        let archived_ref = rkyv::access::<ArchivedPyroRow, RancorError>(view)
            .map_err(|e| PyroError::validation(CapturedError::new(e)))?;
        let row = PyroRow::from(archived_ref);
        Ok(row)
    }
}

impl<'a> PyroValue<'a> {
    /// Converts this into an rkyv formatted pyrovec.
    pub fn to_wire(&self) -> Result<PyroVec, PyroError> {
        let bytes = rkyv::to_bytes::<RancorError>(self)
            .map_err(|e| PyroError::serialization(CapturedError::new(e)))?;
        let mut vec = PyroVec::with_capacity(bytes.len());
        vec.extend_from_slice(&bytes);
        Ok(vec)
    }

    ///
    pub fn parse_wire(view: &'a PyroRef<'a>) -> Result<Self, PyroError> {
        let archived_ref = rkyv::access::<ArchivedPyroValue, RancorError>(view)
            .map_err(|e| PyroError::validation(CapturedError::new(e)))?;
        let row = PyroValue::from(archived_ref);
        Ok(row)
    }
}

#[cfg(test)]
mod tests {
    use crate::format::header::PyroData;

    use super::*;

    #[test]
    fn test_pyro_row_owned_ship_roundtrip() {
        let row = PyroRow::from([
            ("id", PyroValue::from(42i32)),
            ("name", PyroValue::from("hello")),
            ("data", PyroValue::from(&[1.0f64, 2.0, 3.0][..])),
        ])
        .into_owned();

        // Ship
        let vec = row.to_wire().expect("ship failed");
        assert!(vec.len() > 0);

        // Expose
        let view = vec.py_ref();
        let recovered = PyroRow::parse_wire(&view).expect("expose failed");
        assert_eq!(recovered, row);
        drop(vec);
    }

    #[test]
    fn test_pyro_value_owned_ship_roundtrip() {
        let val = PyroValue::from(42i32).into_owned();

        let vec = val.to_wire().expect("ship failed");
        let view = vec.py_ref();
        let recovered = PyroValue::parse_wire(&view).expect("expose failed");
        assert_eq!(recovered, val);
        drop(vec);
    }

    #[test]
    fn test_nested_row_roundtrip() {
        let inner = PyroRow::from([("x", PyroValue::from(99i64))]).into_owned();

        let outer = PyroRow::from([
            ("id", PyroValue::from(1i32)),
            ("nested", PyroValue::Group(inner)),
        ])
        .into_owned();

        let vec = outer.to_wire().expect("ship failed");
        let view = vec.py_ref();
        let recovered = PyroRow::parse_wire(&view).expect("expose failed");
        assert_eq!(recovered, outer);
        drop(vec);
    }

    #[test]
    fn test_complex_value_roundtrip() {
        let val = PyroValue::List(vec![
            PyroValue::from(1i32).into_owned(),
            PyroValue::from("hello").into_owned(),
            PyroValue::Null,
        ]);

        let vec = val.to_wire().expect("ship failed");
        let view = vec.py_ref();
        let recovered = PyroValue::parse_wire(&view).expect("expose failed");
        assert_eq!(recovered, val);
        drop(vec);
    }
}
