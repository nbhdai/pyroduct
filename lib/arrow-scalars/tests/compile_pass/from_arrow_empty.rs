//! Test FromRow with empty struct (edge case)

use arrow_scalars::{FromRow, DeepRef, ArrowRow};

#[derive(FromRow, DeepRef)]
struct Empty {}

fn main() {
    let row = ArrowRow::new();
    let _e = EmptyRef::from_row(&row).unwrap();
    
    // Should compile and work with empty struct
}