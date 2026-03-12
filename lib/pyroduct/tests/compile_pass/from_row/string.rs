//! Test FromRow with String fields (mapped to &str)

use pyroduct::format::{DeepRef, FromRow, PyroRow, PyroValue, RefFromRow};

#[derive(FromRow, RefFromRow, DeepRef)]
struct WithString {
    name: String,
    description: String,
}

fn main() {
    let row = PyroRow::from([
        ("name", PyroValue::from("Alice")),
        ("description", PyroValue::from("A test user")),
    ]);

    let s = WithStringRef::try_from(&row).unwrap();

    // Should be &str, not String
    assert_eq!(s.name, "Alice");
    assert_eq!(s.description, "A test user");

    // Verify it's actually a reference
    let _: &str = s.name;
}
