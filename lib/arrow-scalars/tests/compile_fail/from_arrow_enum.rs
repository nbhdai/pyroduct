//! FromRow, DeepRef should only work on structs, not enums

use arrow_scalars::{FromRow, DeepRef};

#[derive(FromRow, DeepRef)]
enum NotAllowed {
    Variant1,
    Variant2,
}

fn main() {}