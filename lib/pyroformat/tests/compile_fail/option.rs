//! Test #[derive(Layout, Writer, Reader)] with Option fields

use pyroformat::{Layout, Writer, Reader};

#[derive(Layout, Writer, Reader)]
struct WithOption {
    optional_str: Option<String>,
}

fn main() {}
