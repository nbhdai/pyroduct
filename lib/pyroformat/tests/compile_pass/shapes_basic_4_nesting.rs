//! Every derived struct shape from `tests/value_match/basic_4_nesting_u32.rs`, minus
//! the golden-fixture plumbing: each must derive `Layout`, `Writer`
//! and `Reader`. Generated from the value_match file — keep in sync.

use pyroformat::{Layout, Reader, Writer};

fn assert_pyro_traits<T: Layout + Writer + Reader>() {}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PBasic4Str32 {
    a: Vec<Vec<Vec<Vec<String>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PBasic4Str16 {
    a: Vec<Vec<Vec<Vec<String>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PBasic4Bytes32 {
    a: Vec<Vec<Vec<Vec<Vec<u8>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PBasic4Bytes16 {
    a: Vec<Vec<Vec<Vec<Vec<u8>>>>>,
}

fn main() {
    assert_pyro_traits::<PBasic4Str32>();
    assert_pyro_traits::<PBasic4Str16>();
    assert_pyro_traits::<PBasic4Bytes32>();
    assert_pyro_traits::<PBasic4Bytes16>();
}
