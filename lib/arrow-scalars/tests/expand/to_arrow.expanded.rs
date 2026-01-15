//! Expansion test for ToRow derive
use arrow_scalars::ToRow;
struct TestStruct {
    id: u32,
    name: String,
    active: bool,
}
impl ::arrow_scalars::ToRow for TestStruct {
    fn to_row(&self) -> ::arrow_scalars::ArrowRow<'_> {
        ::arrow_scalars::ArrowRow::from([
            ("id", ::arrow_scalars::ArrowValue::from(&self.id)),
            ("name", ::arrow_scalars::ArrowValue::from(&self.name)),
            ("active", ::arrow_scalars::ArrowValue::from(&self.active)),
        ])
    }
}
