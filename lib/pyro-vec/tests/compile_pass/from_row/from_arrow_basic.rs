//! Test FromRow with basic primitive types

use pyro_vec::{FromRow, DeepRef, ArrowValue, ArrowRow};

#[derive(FromRow, DeepRef)]
struct SimpleStruct {
    id: u32,
    count: i32,
    active: bool,
    score: f64,
}

fn main() {
    let row = ArrowRow::from([
        ("id", ArrowValue::U32(42)),
        ("count", ArrowValue::I32(-10)),
        ("active", ArrowValue::Bool(true)),
        ("score", ArrowValue::F64(98.5)),
    ]);

    let s = SimpleStructRef::from_row(&row).unwrap();
    
    assert_eq!(s.id, 42);
    assert_eq!(s.count, -10);
    assert_eq!(s.active, true);
    assert_eq!(s.score, 98.5);
}