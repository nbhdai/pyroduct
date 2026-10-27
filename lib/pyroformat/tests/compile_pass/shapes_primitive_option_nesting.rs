//! Every derived struct shape from `tests/value_match/primitive_option_nesting.rs`, minus
//! the golden-fixture plumbing: each must derive `Layout`, `Writer`
//! and `Reader`. Generated from the value_match file — keep in sync.

use pyroformat::{Layout, Reader, Writer};

fn assert_pyro_traits<T: Layout + Writer + Reader>() {}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PListOfListOptionF32W32 {
    a: Vec<Vec<Option<f32>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PListOfListOptionF32W16 {
    a: Vec<Vec<Option<f32>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PListOfListOfListOptionU8W32 {
    a: Vec<Vec<Vec<Option<u8>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PListOfListOfListOptionU8W16 {
    a: Vec<Vec<Vec<Option<u8>>>>,
}

fn main() {
    assert_pyro_traits::<PListOfListOptionF32W32>();
    assert_pyro_traits::<PListOfListOptionF32W16>();
    assert_pyro_traits::<PListOfListOfListOptionU8W32>();
    assert_pyro_traits::<PListOfListOfListOptionU8W16>();
}
