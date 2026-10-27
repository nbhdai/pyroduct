//! Every derived struct shape from `tests/value_match/basic_u32.rs`, minus
//! the golden-fixture plumbing: each must derive `Layout`, `Writer`
//! and `Reader`. Generated from the value_match file — keep in sync.

use pyroformat::{Layout, Reader, Writer};

fn assert_pyro_traits<T: Layout + Writer + Reader>() {}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PStr32 {
    a: String,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PStr16 {
    a: String,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PBytes32 {
    a: Vec<u8>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PBytes16 {
    a: Vec<u8>,
}

fn main() {
    assert_pyro_traits::<PStr32>();
    assert_pyro_traits::<PStr16>();
    assert_pyro_traits::<PBytes32>();
    assert_pyro_traits::<PBytes16>();
}
