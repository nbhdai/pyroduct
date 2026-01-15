//! Test ToRow with Option fields

use arrow_scalars::{ToRow, ArrowValue};

#[derive(ToRow)]
struct WithOption {
    required: i32,
    optional_num: Option<i32>,
    optional_str: Option<String>,
}

fn main() {
    // Test with Some values
    let data1 = WithOption {
        required: 100,
        optional_num: Some(200),
        optional_str: Some("present".to_string()),
    };
    
    let row1 = data1.to_row();
    assert_eq!(row1.get("required"), Some(&ArrowValue::I32(100)));
    assert_eq!(row1.get("optional_num"), Some(&ArrowValue::I32(200)));
    assert_eq!(row1.get("optional_str"), Some(&ArrowValue::from("present")));
    
    // Test with None values
    let data2 = WithOption {
        required: 100,
        optional_num: None,
        optional_str: None,
    };
    
    let row2 = data2.to_row();
    assert_eq!(row2.get("required"), Some(&ArrowValue::I32(100)));
    assert_eq!(row2.get("optional_num"), Some(&ArrowValue::Null));
    assert_eq!(row2.get("optional_str"), Some(&ArrowValue::Null));
}