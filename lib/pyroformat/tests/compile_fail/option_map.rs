//! `Option<HashMap<K, V>>` is not supported and should fail cleanly.

use pyroformat::{Layout, Writer, Reader};
use std::collections::HashMap;

#[derive(Layout, Writer, Reader)]
struct WithOptionMap {
    value: Option<HashMap<String, u32>>,
}

fn main() {}
