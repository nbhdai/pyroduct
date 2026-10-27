//! Every derived struct shape from `tests/value_match/group_5_nesting_u32.rs`, minus
//! the golden-fixture plumbing: each must derive `Layout`, `Writer`
//! and `Reader`. Generated from the value_match file — keep in sync.

use pyroformat::{Layout, Reader, Writer};

fn assert_pyro_traits<T: Layout + Writer + Reader>() {}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroup5PrimitiveInner32 {
    a: bool,
    b: bool,
    c: i32,
    d: Option<bool>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroup5PrimitiveInner16 {
    a: bool,
    b: bool,
    c: i32,
    d: Option<bool>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroup5Primitive32 {
    a: Vec<Vec<Vec<Vec<Vec<GGroup5PrimitiveInner32>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroup5Primitive16 {
    a: Vec<Vec<Vec<Vec<Vec<GGroup5PrimitiveInner16>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroup5BlobInner32 {
    s: String,
    t: Vec<u8>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroup5BlobInner16 {
    s: String,
    t: Vec<u8>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroup5Blob32 {
    a: Vec<Vec<Vec<Vec<Vec<GGroup5BlobInner32>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroup5Blob16 {
    a: Vec<Vec<Vec<Vec<Vec<GGroup5BlobInner16>>>>>,
}

fn main() {
    assert_pyro_traits::<GGroup5PrimitiveInner32>();
    assert_pyro_traits::<GGroup5PrimitiveInner16>();
    assert_pyro_traits::<GGroup5Primitive32>();
    assert_pyro_traits::<GGroup5Primitive16>();
    assert_pyro_traits::<GGroup5BlobInner32>();
    assert_pyro_traits::<GGroup5BlobInner16>();
    assert_pyro_traits::<GGroup5Blob32>();
    assert_pyro_traits::<GGroup5Blob16>();
}
