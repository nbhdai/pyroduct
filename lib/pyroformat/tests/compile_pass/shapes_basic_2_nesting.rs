//! Every derived struct shape from `tests/value_match/basic_2_nesting_u32.rs`, minus
//! the golden-fixture plumbing: each must derive `Layout`, `Writer`
//! and `Reader`. Generated from the value_match file — keep in sync.

use pyroformat::{Layout, Reader, Writer};

fn assert_pyro_traits<T: Layout + Writer + Reader>() {}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PBasic2Str32 {
    a: Vec<Vec<String>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PBasic2Str16 {
    a: Vec<Vec<String>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PBasic2Bytes32 {
    a: Vec<Vec<Vec<u8>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PBasic2Bytes16 {
    a: Vec<Vec<Vec<u8>>>,
}

fn main() {
    assert_pyro_traits::<PBasic2Str32>();
    assert_pyro_traits::<PBasic2Str16>();
    assert_pyro_traits::<PBasic2Bytes32>();
    assert_pyro_traits::<PBasic2Bytes16>();
}
