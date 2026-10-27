//! Every derived struct shape from `tests/value_match/map_nested_u32.rs`, minus
//! the golden-fixture plumbing: each must derive `Layout`, `Writer`
//! and `Reader`. Generated from the value_match file — keep in sync.

use std::collections::BTreeMap;
use pyroformat::{Layout, Reader, Writer};

fn assert_pyro_traits<T: Layout + Writer + Reader>() {}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PListMapU8U8W32 {
    a: Vec<BTreeMap<u8, u8>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PListMapU8U8W16 {
    a: Vec<BTreeMap<u8, u8>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PListListMapU8U8W32 {
    a: Vec<Vec<BTreeMap<u8, u8>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PListListMapU8U8W16 {
    a: Vec<Vec<BTreeMap<u8, u8>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PListListListMapU8U8W32 {
    a: Vec<Vec<Vec<BTreeMap<u8, u8>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PListListListMapU8U8W16 {
    a: Vec<Vec<Vec<BTreeMap<u8, u8>>>>,
}

fn main() {
    assert_pyro_traits::<PListMapU8U8W32>();
    assert_pyro_traits::<PListMapU8U8W16>();
    assert_pyro_traits::<PListListMapU8U8W32>();
    assert_pyro_traits::<PListListMapU8U8W16>();
    assert_pyro_traits::<PListListListMapU8U8W32>();
    assert_pyro_traits::<PListListListMapU8U8W16>();
}
