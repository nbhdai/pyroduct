//! `#[pyro(allow(vec_bool))]` only applies to a `Vec<bool>` field (at any
//! list depth) —
//! using it on any other field shape is rejected as misapplied.

use pyroformat::{Layout, Writer, Reader};

#[derive(Layout, Writer, Reader)]
struct MisusedAllow {
    #[pyro(allow(vec_bool))]
    count: u32,
}

fn main() {}
