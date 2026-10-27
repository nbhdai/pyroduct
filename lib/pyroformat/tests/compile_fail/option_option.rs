//! `Option<Option<T>>` is not supported and should fail cleanly.

use pyroformat::{Layout, Writer, Reader};

#[derive(Layout, Writer, Reader)]
struct WithOptionOption {
    value: Option<Option<u32>>,
}

fn main() {}
