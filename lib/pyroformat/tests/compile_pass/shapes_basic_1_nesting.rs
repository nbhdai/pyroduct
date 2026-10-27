//! Every derived struct shape from `tests/value_match/basic_1_nesting_u32.rs`, minus
//! the golden-fixture plumbing: each must derive `Layout`, `Writer`
//! and `Reader`. Generated from the value_match file — keep in sync.

use pyroformat::{Layout, Reader, Writer};

fn assert_pyro_traits<T: Layout + Writer + Reader>() {}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PBasic1Str32 {
    a: Vec<String>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PBasic1Str16 {
    a: Vec<String>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PBasic1Bytes32 {
    a: Vec<Vec<u8>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PBasic1Bytes16 {
    a: Vec<Vec<u8>>,
}

fn main() {
    assert_pyro_traits::<PBasic1Str32>();
    assert_pyro_traits::<PBasic1Str16>();
    assert_pyro_traits::<PBasic1Bytes32>();
    assert_pyro_traits::<PBasic1Bytes16>();
}
