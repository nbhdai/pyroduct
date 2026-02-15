//! Test ToRow with Option fields

use pyroduct::ToRow;

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
    
    let _row1 = data1.to_row();
    
    // Test with None values
    let data2 = WithOption {
        required: 100,
        optional_num: None,
        optional_str: None,
    };
    
    let _row2 = data2.to_row();
}