//! Expansion test for ToRow derive
use pyroduct::ToRow;
struct TestStruct {
    id: u32,
    name: String,
    active: bool,
}
impl ::pyroduct::arrow_scalars::ToRow for TestStruct {
    fn to_row(&self) -> ::pyroduct::arrow_scalars::ArrowRow<'_> {
        ::pyroduct::arrow_scalars::ArrowRow::from([
            ("id", ::pyroduct::arrow_scalars::ArrowValue::from(&self.id)),
            ("name", ::pyroduct::arrow_scalars::ArrowValue::from(&self.name)),
            ("active", ::pyroduct::arrow_scalars::ArrowValue::from(&self.active)),
        ])
    }
}
