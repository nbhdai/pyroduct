//! Tuple fields are not supported and should fail cleanly.

use pyroformat::{Layout, Writer, Reader};

#[derive(Layout, Writer, Reader)]
struct WithTuple {
    value: (u32, String),
}

fn main() {}
