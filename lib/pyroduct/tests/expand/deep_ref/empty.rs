//! Test FromRow with empty struct (edge case)

use pyroduct::{DeepRef};

#[derive(DeepRef)]
struct Empty {}

fn main() {
    let empty = Empty {};
    let _e = empty.as_deep_ref();
    
    // Should compile and work with empty struct
}