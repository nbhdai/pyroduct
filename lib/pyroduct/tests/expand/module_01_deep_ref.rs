//! Expansion test for DeepRef derive

use pyroduct::DeepRef;

#[derive(DeepRef)]
struct TestStruct {
    id: u32,
    name: String,
}