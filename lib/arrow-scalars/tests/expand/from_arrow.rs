//! Expansion test for FromRow derive

use arrow_scalars::FromRow;

#[derive(FromRow)]
struct TestStruct {
    id: u32,
    name: String,
    scores: Vec<i32>,
}