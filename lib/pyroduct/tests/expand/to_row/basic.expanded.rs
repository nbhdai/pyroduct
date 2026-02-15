//! Test ToRow with basic types
use pyroduct::{ToRow, PyroValue};
struct BasicData {
    id: u32,
    name: String,
    active: bool,
    score: f64,
}
impl ::pyroduct::ToRow for BasicData {
    fn to_row(&self) -> ::pyroduct::PyroRow<'_> {
        ::pyroduct::PyroRow::from([
            ("id", ::pyroduct::PyroValue::from(&self.id)),
            ("name", ::pyroduct::PyroValue::from(&self.name)),
            ("active", ::pyroduct::PyroValue::from(&self.active)),
            ("score", ::pyroduct::PyroValue::from(&self.score)),
        ])
    }
}
fn main() {
    let data = BasicData {
        id: 99,
        name: "test".to_string(),
        active: true,
        score: 85.5,
    };
    let row = data.to_row();
    match (&row.get("id"), &Some(&PyroValue::U32(99))) {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
}
