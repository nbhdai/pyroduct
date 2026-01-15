//! Test FromRow with Option fields

use arrow_scalars::{FromRow, DeepRef, ArrowValue, ArrowRow};

#[derive(FromRow, DeepRef)]
struct WithOption {
    required: i32,
    optional_num: Option<i32>,
    optional_str: Option<String>,
}

fn main() {
    // Test with Some values
    let row = ArrowRow::from([
        ("required", ArrowValue::I32(100)),
        ("optional_num", ArrowValue::I32(200)),
        ("optional_str", ArrowValue::from("present")),
    ]);

    let s = WithOptionRef::from_row(&row).unwrap();
    
    assert_eq!(s.required, 100);
    assert_eq!(s.optional_num, Some(200));
    assert_eq!(s.optional_str, Some("present"));
    
    // Test with None values
    let row2 = ArrowRow::from([
        ("required", ArrowValue::I32(100)),
        ("optional_num", ArrowValue::Null),
        ("optional_str", ArrowValue::Null),
    ]);

    let s2 = WithOptionRef::from_row(&row2).unwrap();
    
    assert_eq!(s2.required, 100);
    assert_eq!(s2.optional_num, None);
    assert_eq!(s2.optional_str, None);
}