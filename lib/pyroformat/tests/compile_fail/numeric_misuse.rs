//! `#[pyro(numeric)]` only makes sense on a `u8`-based field.

use pyroformat::{Layout, Writer, Reader};

#[derive(Layout, Writer, Reader)]
struct BadNumeric {
    #[pyro(numeric)]
    name: String,
}

fn main() {}
