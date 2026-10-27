//! Every derived struct shape from `tests/value_match/group_3_nesting_u32.rs`, minus
//! the golden-fixture plumbing: each must derive `Layout`, `Writer`
//! and `Reader`. Generated from the value_match file — keep in sync.

use pyroformat::value::Time;
use pyroformat::{Layout, Reader, Writer};

fn assert_pyro_traits<T: Layout + Writer + Reader>() {}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroup3PrimitiveInner32 {
    a: Time,
    b: u8,
    c: Option<i32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroup3PrimitiveInner16 {
    a: Time,
    b: u8,
    c: Option<i32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroup3Primitive32 {
    a: Vec<Vec<Vec<GGroup3PrimitiveInner32>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroup3Primitive16 {
    a: Vec<Vec<Vec<GGroup3PrimitiveInner16>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroup3BlobInner32 {
    id: u32,
    s: String,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroup3BlobInner16 {
    id: u32,
    s: String,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroup3Blob32 {
    a: Vec<Vec<Vec<GGroup3BlobInner32>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroup3Blob16 {
    a: Vec<Vec<Vec<GGroup3BlobInner16>>>,
}

fn main() {
    assert_pyro_traits::<GGroup3PrimitiveInner32>();
    assert_pyro_traits::<GGroup3PrimitiveInner16>();
    assert_pyro_traits::<GGroup3Primitive32>();
    assert_pyro_traits::<GGroup3Primitive16>();
    assert_pyro_traits::<GGroup3BlobInner32>();
    assert_pyro_traits::<GGroup3BlobInner16>();
    assert_pyro_traits::<GGroup3Blob32>();
    assert_pyro_traits::<GGroup3Blob16>();
}
