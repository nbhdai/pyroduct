//! Every derived struct shape from `tests/value_match/group_option_1_nesting_u32.rs`, minus
//! the golden-fixture plumbing: each must derive `Layout`, `Writer`
//! and `Reader`. Generated from the value_match file — keep in sync.

use pyroformat::{Layout, Reader, Writer};

fn assert_pyro_traits<T: Layout + Writer + Reader>() {}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroupOption1PrimitiveInner32 {
    a: u8,
    b: u32,
    c: Option<u16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroupOption1PrimitiveInner16 {
    a: u8,
    b: u32,
    c: Option<u16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroupOption1Primitive32 {
    a: Vec<Option<GGroupOption1PrimitiveInner32>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroupOption1Primitive16 {
    a: Vec<Option<GGroupOption1PrimitiveInner16>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroupOption1BlobInner32 {
    s: String,
    n: u16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroupOption1BlobInner16 {
    s: String,
    n: u16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroupOption1Blob32 {
    a: Vec<Option<GGroupOption1BlobInner32>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroupOption1Blob16 {
    a: Vec<Option<GGroupOption1BlobInner16>>,
}

fn main() {
    assert_pyro_traits::<GGroupOption1PrimitiveInner32>();
    assert_pyro_traits::<GGroupOption1PrimitiveInner16>();
    assert_pyro_traits::<GGroupOption1Primitive32>();
    assert_pyro_traits::<GGroupOption1Primitive16>();
    assert_pyro_traits::<GGroupOption1BlobInner32>();
    assert_pyro_traits::<GGroupOption1BlobInner16>();
    assert_pyro_traits::<GGroupOption1Blob32>();
    assert_pyro_traits::<GGroupOption1Blob16>();
}
