//! Every derived struct shape from `tests/value_match/group_option_2_nesting_u32.rs`, minus
//! the golden-fixture plumbing: each must derive `Layout`, `Writer`
//! and `Reader`. Generated from the value_match file — keep in sync.

use pyroformat::{Layout, Reader, Writer};

fn assert_pyro_traits<T: Layout + Writer + Reader>() {}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroupOption2PrimitiveInner32 {
    a: i16,
    b: bool,
    c: f64,
    d: Option<u8>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroupOption2PrimitiveInner16 {
    a: i16,
    b: bool,
    c: f64,
    d: Option<u8>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroupOption2Primitive32 {
    a: Vec<Vec<Option<GGroupOption2PrimitiveInner32>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroupOption2Primitive16 {
    a: Vec<Vec<Option<GGroupOption2PrimitiveInner16>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroupOption2BlobInner32 {
    b: Vec<u8>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroupOption2BlobInner16 {
    b: Vec<u8>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroupOption2Blob32 {
    a: Vec<Vec<Option<GGroupOption2BlobInner32>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroupOption2Blob16 {
    a: Vec<Vec<Option<GGroupOption2BlobInner16>>>,
}

fn main() {
    assert_pyro_traits::<GGroupOption2PrimitiveInner32>();
    assert_pyro_traits::<GGroupOption2PrimitiveInner16>();
    assert_pyro_traits::<GGroupOption2Primitive32>();
    assert_pyro_traits::<GGroupOption2Primitive16>();
    assert_pyro_traits::<GGroupOption2BlobInner32>();
    assert_pyro_traits::<GGroupOption2BlobInner16>();
    assert_pyro_traits::<GGroupOption2Blob32>();
    assert_pyro_traits::<GGroupOption2Blob16>();
}
