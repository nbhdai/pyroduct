//! Every derived struct shape from `tests/value_match/group_1_nesting_u32.rs`, minus
//! the golden-fixture plumbing: each must derive `Layout`, `Writer`
//! and `Reader`. Generated from the value_match file — keep in sync.

use pyroformat::{Layout, Reader, Writer};

fn assert_pyro_traits<T: Layout + Writer + Reader>() {}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroup1PrimitiveInner32 {
    a: u8,
    b: u32,
    c: Option<u16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroup1PrimitiveInner16 {
    a: u8,
    b: u32,
    c: Option<u16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroup1Primitive32 {
    a: Vec<GGroup1PrimitiveInner32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroup1Primitive16 {
    a: Vec<GGroup1PrimitiveInner16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroup1BlobInner32 {
    s: String,
    n: u16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroup1BlobInner16 {
    s: String,
    n: u16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroup1Blob32 {
    a: Vec<GGroup1BlobInner32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroup1Blob16 {
    a: Vec<GGroup1BlobInner16>,
}

fn main() {
    assert_pyro_traits::<GGroup1PrimitiveInner32>();
    assert_pyro_traits::<GGroup1PrimitiveInner16>();
    assert_pyro_traits::<GGroup1Primitive32>();
    assert_pyro_traits::<GGroup1Primitive16>();
    assert_pyro_traits::<GGroup1BlobInner32>();
    assert_pyro_traits::<GGroup1BlobInner16>();
    assert_pyro_traits::<GGroup1Blob32>();
    assert_pyro_traits::<GGroup1Blob16>();
}
