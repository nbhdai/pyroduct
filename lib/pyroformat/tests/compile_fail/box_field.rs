//! `Box<T>` fields are not supported and should fail cleanly.

use pyroformat::{Layout, Writer, Reader};

#[derive(Layout, Writer, Reader)]
struct WithBox {
    value: Box<u32>,
}

fn main() {}
