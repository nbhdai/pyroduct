//! Expansion test for DeepRef derive

use arrow_scalars::DeepRef;

#[derive(DeepRef)]
struct TestStruct {
    id: u32,
    name: String,
}