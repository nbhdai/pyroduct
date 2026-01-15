//! Expansion test for ToRow derive

use arrow_scalars::ToRow;

#[derive(ToRow)]
struct TestStruct {
    id: u32,
    name: String,
    active: bool,
}