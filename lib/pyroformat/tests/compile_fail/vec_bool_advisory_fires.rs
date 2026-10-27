//! A plain `Vec<bool>` field (no `#[pyro(allow(vec_bool))]`) triggers the
//! `Vec<bool>` advisory: a `#[deprecated]` warning nudging toward
//! `pyroformat::BitVec`. `#![deny(warnings)]` turns that warning into a
//! hard compile error, proving the advisory actually fires.

#![deny(warnings)]

use pyroformat::{Layout, Writer, Reader};

#[derive(Layout, Writer, Reader)]
struct WithUnsuppressedVecBool {
    flags: Vec<bool>,
}

fn main() {}
