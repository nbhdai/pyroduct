//! `Rc<T>` fields are not supported and should fail cleanly.

use pyroformat::{Layout, Writer, Reader};
use std::rc::Rc;

#[derive(Layout, Writer, Reader)]
struct WithRc {
    value: Rc<u32>,
}

fn main() {}
