//! `Arc<T>` fields are not supported and should fail cleanly.

use pyroformat::{Layout, Writer, Reader};
use std::sync::Arc;

#[derive(Layout, Writer, Reader)]
struct WithArc {
    value: Arc<u32>,
}

fn main() {}
