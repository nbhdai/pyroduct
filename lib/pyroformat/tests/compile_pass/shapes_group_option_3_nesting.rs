//! Every derived struct shape from `tests/value_match/group_option_3_nesting_u32.rs`, minus
//! the golden-fixture plumbing: each must derive `Layout`, `Writer`
//! and `Reader`. Generated from the value_match file — keep in sync.

use pyroformat::value::Time;
use pyroformat::{Layout, Reader, Writer};

fn assert_pyro_traits<T: Layout + Writer + Reader>() {}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroupOption3PrimitiveInner32 {
    a: Time,
    b: u8,
    c: Option<i32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroupOption3PrimitiveInner16 {
    a: Time,
    b: u8,
    c: Option<i32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroupOption3Primitive32 {
    a: Vec<Vec<Vec<Option<GGroupOption3PrimitiveInner32>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroupOption3Primitive16 {
    a: Vec<Vec<Vec<Option<GGroupOption3PrimitiveInner16>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroupOption3BlobInner32 {
    id: u32,
    s: String,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroupOption3BlobInner16 {
    id: u32,
    s: String,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroupOption3Blob32 {
    a: Vec<Vec<Vec<Option<GGroupOption3BlobInner32>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroupOption3Blob16 {
    a: Vec<Vec<Vec<Option<GGroupOption3BlobInner16>>>>,
}

fn main() {
    assert_pyro_traits::<GGroupOption3PrimitiveInner32>();
    assert_pyro_traits::<GGroupOption3PrimitiveInner16>();
    assert_pyro_traits::<GGroupOption3Primitive32>();
    assert_pyro_traits::<GGroupOption3Primitive16>();
    assert_pyro_traits::<GGroupOption3BlobInner32>();
    assert_pyro_traits::<GGroupOption3BlobInner16>();
    assert_pyro_traits::<GGroupOption3Blob32>();
    assert_pyro_traits::<GGroupOption3Blob16>();
}
