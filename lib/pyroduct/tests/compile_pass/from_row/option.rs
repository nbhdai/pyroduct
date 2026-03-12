//! Test FromRow with Option fields

use pyroduct::format::{DeepRef, FromRow, PyroRow, PyroValue, RefFromRow};

#[derive(FromRow, RefFromRow, DeepRef)]
struct WithOption {
    required: i32,
    optional_num: Option<i32>,
    optional_str: Option<String>,
}

fn main() {
    // Test with Some values
    let row = PyroRow::from([
        ("required", PyroValue::I32(100)),
        ("optional_num", PyroValue::I32(200)),
        ("optional_str", PyroValue::from("present")),
    ]);

    let s = WithOptionRef::try_from(&row).unwrap();

    assert_eq!(s.required, 100);
    assert_eq!(s.optional_num, Some(200));
    assert_eq!(s.optional_str, Some("present"));

    // Test with None values
    let row2 = PyroRow::from([
        ("required", PyroValue::I32(100)),
        ("optional_num", PyroValue::Null),
        ("optional_str", PyroValue::Null),
    ]);

    let s2 = WithOptionRef::try_from(&row2).unwrap();

    assert_eq!(s2.required, 100);
    assert_eq!(s2.optional_num, None);
    assert_eq!(s2.optional_str, None);
}
