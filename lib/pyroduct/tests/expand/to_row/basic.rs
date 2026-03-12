//! Test ToRow with basic types

use pyroduct::format::{PyroValue, ToRow};

#[derive(ToRow)]
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

    assert_eq!(row.get("id"), Some(&PyroValue::U32(99)));
}
