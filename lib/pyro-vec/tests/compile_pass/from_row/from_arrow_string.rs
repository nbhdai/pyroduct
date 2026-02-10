//! Test FromRow with String fields (mapped to &str)

use pyro_vec::{FromRow, DeepRef, ArrowValue, ArrowRow};

#[derive(FromRow, DeepRef)]
struct WithString {
    name: String,
    description: String,
}

fn main() {
    let row = ArrowRow::from([
        ("name", ArrowValue::from("Alice")),
        ("description", ArrowValue::from("A test user")),
    ]);

    let s = WithStringRef::from_row(&row).unwrap();
    
    // Should be &str, not String
    assert_eq!(s.name, "Alice");
    assert_eq!(s.description, "A test user");
    
    // Verify it's actually a reference
    let _: &str = s.name;
}