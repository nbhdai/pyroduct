//! `Option<Vec<T>>` is not supported (an empty `Vec` already represents
//! "no value") and should fail cleanly.

use pyroformat::{Reader, Layout, Writer};

#[derive(Layout, Writer, Reader)]
struct WithOptionVec {
    value: Option<Vec<u32>>,
}

fn main() {}
