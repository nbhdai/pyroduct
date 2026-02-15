//! Test FromRow with basic primitive types

use pyroduct::{FromRow, RefFromRow, DeepRef, PyroValue, PyroRow};

#[derive(FromRow, RefFromRow, DeepRef)]
struct SimpleStruct {
    id: u32,
    count: i32,
    active: bool,
    score: f64,
}

fn main() {
    let row = PyroRow::from([
        ("id", PyroValue::U32(42)),
        ("count", PyroValue::I32(-10)),
        ("active", PyroValue::Bool(true)),
        ("score", PyroValue::F64(98.5)),
    ]);

    let s = SimpleStructRef::try_from(&row).unwrap();
    
    assert_eq!(s.id, 42);
    assert_eq!(s.count, -10);
    assert_eq!(s.active, true);
    assert_eq!(s.score, 98.5);
}