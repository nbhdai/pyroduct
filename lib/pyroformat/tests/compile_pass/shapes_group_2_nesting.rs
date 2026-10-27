//! Every derived struct shape from `tests/value_match/group_2_nesting_u32.rs`, minus
//! the golden-fixture plumbing: each must derive `Layout`, `Writer`
//! and `Reader`. Generated from the value_match file — keep in sync.

use pyroformat::{Layout, Reader, Writer};

fn assert_pyro_traits<T: Layout + Writer + Reader>() {}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroup2PrimitiveInner32 {
    a: i16,
    b: bool,
    c: f64,
    d: Option<u8>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroup2PrimitiveInner16 {
    a: i16,
    b: bool,
    c: f64,
    d: Option<u8>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroup2Primitive32 {
    a: Vec<Vec<GGroup2PrimitiveInner32>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroup2Primitive16 {
    a: Vec<Vec<GGroup2PrimitiveInner16>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroup2BlobInner32 {
    b: Vec<u8>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroup2BlobInner16 {
    b: Vec<u8>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroup2Blob32 {
    a: Vec<Vec<GGroup2BlobInner32>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroup2Blob16 {
    a: Vec<Vec<GGroup2BlobInner16>>,
}

fn main() {
    assert_pyro_traits::<GGroup2PrimitiveInner32>();
    assert_pyro_traits::<GGroup2PrimitiveInner16>();
    assert_pyro_traits::<GGroup2Primitive32>();
    assert_pyro_traits::<GGroup2Primitive16>();
    assert_pyro_traits::<GGroup2BlobInner32>();
    assert_pyro_traits::<GGroup2BlobInner16>();
    assert_pyro_traits::<GGroup2Blob32>();
    assert_pyro_traits::<GGroup2Blob16>();
}
