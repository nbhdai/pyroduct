//! Every derived struct shape from `tests/value_match/primitive_nesting.rs`, minus
//! the golden-fixture plumbing: each must derive `Layout`, `Writer`
//! and `Reader`. Generated from the value_match file — keep in sync.

use pyroformat::{Layout, Reader, Writer};

fn assert_pyro_traits<T: Layout + Writer + Reader>() {}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PListOfStr32 {
    a: Vec<String>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PListOfStr16 {
    a: Vec<String>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PListOfListU64W32 {
    a: Vec<Vec<u64>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PListOfListU64W16 {
    a: Vec<Vec<u64>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PListOfListOptionBool32 {
    a: Vec<Vec<Option<bool>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PListOfListOptionBool16 {
    a: Vec<Vec<Option<bool>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PTripleNestedListBytes32 {
    a: Vec<Vec<Vec<Vec<u8>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PTripleNestedListBytes16 {
    a: Vec<Vec<Vec<Vec<u8>>>>,
}

fn main() {
    assert_pyro_traits::<PListOfStr32>();
    assert_pyro_traits::<PListOfStr16>();
    assert_pyro_traits::<PListOfListU64W32>();
    assert_pyro_traits::<PListOfListU64W16>();
    assert_pyro_traits::<PListOfListOptionBool32>();
    assert_pyro_traits::<PListOfListOptionBool16>();
    assert_pyro_traits::<PTripleNestedListBytes32>();
    assert_pyro_traits::<PTripleNestedListBytes16>();
}
