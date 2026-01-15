//! Test ToRow with basic types

use arrow_scalars::{ToRow, DeepRef, ArrowValue};

#[derive(ToRow, DeepRef)]
struct BasicData {
    id: u32,
    name: String,
    active: bool,
    score: f64,
}

fn main() {
    let data = BasicData {
        id: 99,
        name: "test".to_string(),
        active: true,
        score: 85.5,
    };
    
    let row = data.to_row();
    
    assert_eq!(row.get("id"), Some(&ArrowValue::U32(99)));
    assert_eq!(row.get("name"), Some(&ArrowValue::from("test")));
    assert_eq!(row.get("active"), Some(&ArrowValue::Bool(true)));
    assert_eq!(row.get("score"), Some(&ArrowValue::F64(85.5)));
}