//! Expansion test for FromRow derive
use arrow_scalars::FromRow;
struct TestStruct {
    id: u32,
    name: String,
    scores: Vec<i32>,
}
impl<'a> ::arrow_scalars::FromRow<'a> for TestStructRef<'a> {
    fn from_row(row: &::arrow_scalars::ArrowRow<'a>) -> Result<Self, String> {
        Ok(Self {
            id: <u32 as ::arrow_scalars::FromValue<
                'a,
            >>::from_value(row.get("id").ok_or("Missing Field: id".to_string())?)?,
            name: <&'a str as ::arrow_scalars::FromValue<
                'a,
            >>::from_value(row.get("name").ok_or("Missing Field: name".to_string())?)?,
            scores: <&'a [i32] as ::arrow_scalars::FromValue<
                'a,
            >>::from_value(
                row.get("scores").ok_or("Missing Field: scores".to_string())?,
            )?,
        })
    }
}
