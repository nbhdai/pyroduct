//! Test ToRow with Option fields
use pyroduct::ToRow;
struct WithOption {
    required: i32,
    optional_num: Option<i32>,
    optional_str: Option<String>,
}
impl ::pyroduct::ToRow for WithOption {
    fn to_row(&self) -> ::pyroduct::PyroRow<'_> {
        ::pyroduct::PyroRow::from([
            ("required", ::pyroduct::PyroValue::from(&self.required)),
            ("optional_num", ::pyroduct::PyroValue::from(&self.optional_num)),
            ("optional_str", ::pyroduct::PyroValue::from(&self.optional_str)),
        ])
    }
}
fn main() {
    let data1 = WithOption {
        required: 100,
        optional_num: Some(200),
        optional_str: Some("present".to_string()),
    };
    let _row1 = data1.to_row();
    let data2 = WithOption {
        required: 100,
        optional_num: None,
        optional_str: None,
    };
    let _row2 = data2.to_row();
}
