//! Every derived struct shape from `tests/value_match/group_option_4_nesting_u32.rs`, minus
//! the golden-fixture plumbing: each must derive `Layout`, `Writer`
//! and `Reader`. Generated from the value_match file — keep in sync.

use pyroformat::{Layout, Reader, Writer};

fn assert_pyro_traits<T: Layout + Writer + Reader>() {}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroupOption4PrimitiveInner32 {
    a: f32,
    b: i64,
    c: u16,
    d: Option<f64>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroupOption4PrimitiveInner16 {
    a: f32,
    b: i64,
    c: u16,
    d: Option<f64>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroupOption4Primitive32 {
    a: Vec<Vec<Vec<Vec<Option<GGroupOption4PrimitiveInner32>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroupOption4Primitive16 {
    a: Vec<Vec<Vec<Vec<Option<GGroupOption4PrimitiveInner16>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroupOption4BlobInner32 {
    b: Vec<u8>,
    f: f64,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroupOption4BlobInner16 {
    b: Vec<u8>,
    f: f64,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroupOption4Blob32 {
    a: Vec<Vec<Vec<Vec<Option<GGroupOption4BlobInner32>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroupOption4Blob16 {
    a: Vec<Vec<Vec<Vec<Option<GGroupOption4BlobInner16>>>>>,
}

fn main() {
    assert_pyro_traits::<GGroupOption4PrimitiveInner32>();
    assert_pyro_traits::<GGroupOption4PrimitiveInner16>();
    assert_pyro_traits::<GGroupOption4Primitive32>();
    assert_pyro_traits::<GGroupOption4Primitive16>();
    assert_pyro_traits::<GGroupOption4BlobInner32>();
    assert_pyro_traits::<GGroupOption4BlobInner16>();
    assert_pyro_traits::<GGroupOption4Blob32>();
    assert_pyro_traits::<GGroupOption4Blob16>();
}
