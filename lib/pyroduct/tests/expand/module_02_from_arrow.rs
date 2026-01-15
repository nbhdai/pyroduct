//! Expansion test for FromRow derive

use pyroduct::FromRow;

#[derive(FromRow)]
struct TestStruct {
    id: u32,
    name: String,
    scores: Vec<i32>,
}