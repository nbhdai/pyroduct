//! Bridgeable implementations for PyroRow and types convertible to/from PyroRow.
//!
//! This module provides:
//! 1. `Bridgeable` impl for `PyroRow<'static>` (aka `PyroRowOwned`) via rkyv.
//! 2. A `RowBridgeable` trait + blanket impl for types `T` that implement
//!    `TryFrom<PyroRow<'static>>` and `Into<PyroRow<'static>>`, allowing them
//!    to be shipped/exposed through PyroRow's rkyv serialization.
//!
//! # Usage
//!
//! ```rust,ignore
//! use pyro_vec::value::{PyroRow, PyroValue, PyroRowOwned};
//! use pyro_vec::Bridgeable;
//!
//! // PyroRowOwned is directly Bridgeable:
//! let row = PyroRow::from([
//!     ("id", PyroValue::from(42i32)),
//!     ("name", PyroValue::from("hello")),
//! ]).into_owned();
//!
//! let vec = row.ship().unwrap();
//! ```

use crate::format::{Bridgeable, format::UserHeaderValues, rkyv_8::Rkyv};

use super::{PyroRow, PyroValue};

// =============================================================================
// UserHeaderValues + Bridgeable for PyroRowOwned
// =============================================================================

impl UserHeaderValues for PyroRow<'static> {
    const VERSION: u8 = 0;
}

impl Bridgeable for PyroRow<'static> {
    type Format = Rkyv<PyroRow<'static>>;
}

// =============================================================================
// UserHeaderValues + Bridgeable for PyroValue<'static>
// =============================================================================

impl UserHeaderValues for PyroValue<'static> {
    const VERSION: u8 = 0;
}

impl Bridgeable for PyroValue<'static> {
    type Format = Rkyv<PyroValue<'static>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::{HasReceiver, Receiver, header::PyroHeader, value::PyroRowOwned};

    #[test]
    fn test_pyro_row_owned_ship_roundtrip() {
        let row = PyroRow::from([
            ("id", PyroValue::from(42i32)),
            ("name", PyroValue::from("hello")),
            ("data", PyroValue::from(&[1.0f64, 2.0, 3.0][..])),
        ])
        .into_owned();

        // Ship
        let vec = row.ship().expect("ship failed");
        assert_eq!(vec.status(), Ok(crate::format::header::DataStatus::RkyvValid));
        assert!(vec.len() > 0);

        // Expose
        let typed = PyroRowOwned::expose(vec.view()).expect("expose failed");

        // Receive (deserialize back to owned)
        let mut receiver = typed.receiver();
        let recovered = receiver.receive(&typed).expect("receive failed");

        assert_eq!(recovered, row);
    }

    #[test]
    fn test_pyro_value_owned_ship_roundtrip() {
        let val = PyroValue::from(42i32).into_owned();

        let vec = val.ship().expect("ship failed");
        let typed = PyroValue::<'static>::expose(vec.view()).expect("expose failed");
        let mut receiver = typed.receiver();
        let recovered = receiver.receive(&typed).expect("receive failed");

        assert_eq!(recovered, val);
    }

    #[test]
    fn test_pyro_row_vec_ship_roundtrip() {
        let rows: Vec<PyroRowOwned> = vec![
            PyroRow::from([("a", PyroValue::from(1i32))]).into_owned(),
            PyroRow::from([("b", PyroValue::from(2i32))]).into_owned(),
        ];

        let vec = rows.ship().expect("ship failed");
        let typed: crate::format::TypedBuf<Vec<PyroRow<'static>>> =
            Vec::<PyroRowOwned>::expose(vec.view()).expect("expose failed");
        let mut receiver = typed.receiver();
        let recovered = receiver.receive(&typed).expect("receive failed");

        assert_eq!(recovered.len(), 2);
        assert_eq!(recovered, rows);
    }

    #[test]
    fn test_nested_row_roundtrip() {
        let inner = PyroRow::from([("x", PyroValue::from(99i64))]).into_owned();

        let outer = PyroRow::from([
            ("id", PyroValue::from(1i32)),
            ("nested", PyroValue::Group(inner)),
        ])
        .into_owned();

        let vec = outer.ship().expect("ship failed");
        let typed = PyroRowOwned::expose(vec.view()).expect("expose failed");
        let mut receiver = typed.receiver();
        let recovered = receiver.receive(&typed).expect("receive failed");

        assert_eq!(recovered, outer);
    }

    #[test]
    fn test_complex_value_roundtrip() {
        let val = PyroValue::List(vec![
            PyroValue::from(1i32).into_owned(),
            PyroValue::from("hello").into_owned(),
            PyroValue::Null,
        ]);

        let vec = val.ship().expect("ship failed");
        let typed = PyroValue::<'static>::expose(vec.view()).expect("expose failed");
        let mut receiver = typed.receiver();
        let recovered = receiver.receive(&typed).expect("receive failed");

        assert_eq!(recovered, val);
    }
}
