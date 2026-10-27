//! The `Vec<bool>` advisory fires at every list depth, not only on a plain
//! `Vec<bool>`: here `Vec<Vec<Vec<Vec<Vec<bool>>>>>`. `#![deny(warnings)]` turns the warning
//! into a hard compile error, proving it fires.

#![deny(warnings)]

use pyroformat::{Layout, Reader, Writer};

#[derive(Layout, Writer, Reader)]
struct VecBoolFiveDeep {
    flags: Vec<Vec<Vec<Vec<Vec<bool>>>>>,
}

fn main() {}
