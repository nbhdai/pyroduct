//! ToRow should only work on structs, not enums

use pyroduct::ToRow;

#[derive(ToRow)]
enum NotAllowed {
    Variant1,
    Variant2,
}

fn main() {}