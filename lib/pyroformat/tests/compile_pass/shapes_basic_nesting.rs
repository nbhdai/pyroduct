//! Every derived struct shape from `tests/value_match/basic_nesting_u32.rs`, minus
//! the golden-fixture plumbing: each must derive `Layout`, `Writer`
//! and `Reader`. Generated from the value_match file — keep in sync.

use pyroformat::{Layout, Reader, Writer};

fn assert_pyro_traits<T: Layout + Writer + Reader>() {}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PVecStr32 {
    a: Vec<String>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PVecStr16 {
    a: Vec<String>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PVecBytes32 {
    a: Vec<Vec<u8>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PVecBytes16 {
    a: Vec<Vec<u8>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PVecVecStr32 {
    a: Vec<Vec<String>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PVecVecStr16 {
    a: Vec<Vec<String>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PVecVecBytes32 {
    a: Vec<Vec<Vec<u8>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PVecVecBytes16 {
    a: Vec<Vec<Vec<u8>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GroupStr32 {
    s: String,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GroupStr16 {
    s: String,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PGroupStr32 {
    a: Option<GroupStr32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PGroupStr16 {
    a: Option<GroupStr16>,
}

fn main() {
    assert_pyro_traits::<PVecStr32>();
    assert_pyro_traits::<PVecStr16>();
    assert_pyro_traits::<PVecBytes32>();
    assert_pyro_traits::<PVecBytes16>();
    assert_pyro_traits::<PVecVecStr32>();
    assert_pyro_traits::<PVecVecStr16>();
    assert_pyro_traits::<PVecVecBytes32>();
    assert_pyro_traits::<PVecVecBytes16>();
    assert_pyro_traits::<GroupStr32>();
    assert_pyro_traits::<GroupStr16>();
    assert_pyro_traits::<PGroupStr32>();
    assert_pyro_traits::<PGroupStr16>();
}
