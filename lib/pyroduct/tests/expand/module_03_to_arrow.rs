//! Expansion test for ToRow derive

use pyroduct::ToRow;

#[derive(ToRow)]
struct TestStruct {
    id: u32,
    name: String,
    active: bool,
}