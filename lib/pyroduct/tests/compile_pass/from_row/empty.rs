//! Test FromRow with empty struct (edge case)

use pyroduct::format::{DeepRef, FromRow, PyroRow, RefFromRow};

#[derive(FromRow, RefFromRow, DeepRef)]
struct Empty {}

fn main() {
    let row = PyroRow::new();
    let _e = EmptyRef::try_from(&row).unwrap();

    // Should compile and work with empty struct
}
