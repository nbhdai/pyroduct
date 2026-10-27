//! Every derived struct shape from `tests/value_match/group_nesting_u32.rs`, minus
//! the golden-fixture plumbing: each must derive `Layout`, `Writer`
//! and `Reader`. Generated from the value_match file — keep in sync.

use pyroformat::{Layout, Reader, Writer};

fn assert_pyro_traits<T: Layout + Writer + Reader>() {}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GYInner32 {
    y: u8,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GYInner16 {
    y: u8,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GListGroupY32 {
    a: Vec<GYInner32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GListGroupY16 {
    a: Vec<GYInner16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GNestC32 {
    c: String,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GNestC16 {
    c: String,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GNestOuter32 {
    a: String,
    b: Vec<GNestC32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GNestOuter16 {
    a: String,
    b: Vec<GNestC16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GDoubleListGroupStr32 {
    a: Vec<Vec<GNestOuter32>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GDoubleListGroupStr16 {
    a: Vec<Vec<GNestOuter16>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GLgF64I64Inner32 {
    b1: Vec<Vec<f64>>,
    b2: Vec<Vec<i64>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GLgF64I64Inner16 {
    b1: Vec<Vec<f64>>,
    b2: Vec<Vec<i64>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GListGroupF64I64W32 {
    a: Vec<GLgF64I64Inner32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GListGroupF64I64W16 {
    a: Vec<GLgF64I64Inner16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GTripleD32 {
    d: u32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GTripleD16 {
    d: u32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GTripleC32 {
    c: GTripleD32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GTripleC16 {
    c: GTripleD16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GTripleB32 {
    b: GTripleC32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GTripleB16 {
    b: GTripleC16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GListGroupTripleU32W32 {
    a: Vec<GTripleB32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GListGroupTripleU32W16 {
    a: Vec<GTripleB16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GBytesInner32 {
    a: Vec<u8>,
    b: Vec<Vec<u8>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GBytesInner16 {
    a: Vec<u8>,
    b: Vec<Vec<u8>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GListGroupBytes32 {
    a: Vec<GBytesInner32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GListGroupBytes16 {
    a: Vec<GBytesInner16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GFixedInner32 {
    a: [u8; 16],
    b: Vec<[u16; 8]>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GFixedInner16 {
    a: [u8; 16],
    b: Vec<[u16; 8]>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GListGroupFixedArrays32 {
    a: Vec<GFixedInner32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GListGroupFixedArrays16 {
    a: Vec<GFixedInner16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GTwoU8Inner32 {
    a: u8,
    b: u8,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GTwoU8Inner16 {
    a: u8,
    b: u8,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GDoubleListGroupTwoU8W32 {
    a: Vec<Vec<GTwoU8Inner32>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GDoubleListGroupTwoU8W16 {
    a: Vec<Vec<GTwoU8Inner16>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GTn7C32 {
    c: Vec<Vec<u32>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GTn7C16 {
    c: Vec<Vec<u32>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GTn7B32 {
    b: Vec<GTn7C32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GTn7B16 {
    b: Vec<GTn7C16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GTripleNestedListGroupU32W32 {
    a: Vec<GTn7B32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GTripleNestedListGroupU32W16 {
    a: Vec<GTn7B16>,
}

// `b: Vec<Vec<bool>>` is nested (vec_depth 2), not the plain `Vec<bool>`
// the `#[pyro(allow(vec_bool))]` deprecation-suppression attribute
// targets (`pyroformat-derive/src/parse/structs.rs`'s `is_plain_vec_bool`
// check rejects it on this field) — and it isn't needed here since that
// warning only fires for the plain depth-1 shape anyway.
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GBoolInner32 {
    a: bool,
    #[pyro(allow(vec_bool))]
    b: Vec<Vec<bool>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GBoolInner16 {
    a: bool,
    b: Vec<Vec<bool>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GListGroupBool32 {
    a: Vec<GBoolInner32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GListGroupBool16 {
    a: Vec<GBoolInner16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GTwoStrInner32 {
    a: String,
    b: String,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GTwoStrInner16 {
    a: String,
    b: String,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GListGroupTwoStr32 {
    a: Vec<GTwoStrInner32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GListGroupTwoStr16 {
    a: Vec<GTwoStrInner16>,
}

fn main() {
    assert_pyro_traits::<GYInner32>();
    assert_pyro_traits::<GYInner16>();
    assert_pyro_traits::<GListGroupY32>();
    assert_pyro_traits::<GListGroupY16>();
    assert_pyro_traits::<GNestC32>();
    assert_pyro_traits::<GNestC16>();
    assert_pyro_traits::<GNestOuter32>();
    assert_pyro_traits::<GNestOuter16>();
    assert_pyro_traits::<GDoubleListGroupStr32>();
    assert_pyro_traits::<GDoubleListGroupStr16>();
    assert_pyro_traits::<GLgF64I64Inner32>();
    assert_pyro_traits::<GLgF64I64Inner16>();
    assert_pyro_traits::<GListGroupF64I64W32>();
    assert_pyro_traits::<GListGroupF64I64W16>();
    assert_pyro_traits::<GTripleD32>();
    assert_pyro_traits::<GTripleD16>();
    assert_pyro_traits::<GTripleC32>();
    assert_pyro_traits::<GTripleC16>();
    assert_pyro_traits::<GTripleB32>();
    assert_pyro_traits::<GTripleB16>();
    assert_pyro_traits::<GListGroupTripleU32W32>();
    assert_pyro_traits::<GListGroupTripleU32W16>();
    assert_pyro_traits::<GBytesInner32>();
    assert_pyro_traits::<GBytesInner16>();
    assert_pyro_traits::<GListGroupBytes32>();
    assert_pyro_traits::<GListGroupBytes16>();
    assert_pyro_traits::<GFixedInner32>();
    assert_pyro_traits::<GFixedInner16>();
    assert_pyro_traits::<GListGroupFixedArrays32>();
    assert_pyro_traits::<GListGroupFixedArrays16>();
    assert_pyro_traits::<GTwoU8Inner32>();
    assert_pyro_traits::<GTwoU8Inner16>();
    assert_pyro_traits::<GDoubleListGroupTwoU8W32>();
    assert_pyro_traits::<GDoubleListGroupTwoU8W16>();
    assert_pyro_traits::<GTn7C32>();
    assert_pyro_traits::<GTn7C16>();
    assert_pyro_traits::<GTn7B32>();
    assert_pyro_traits::<GTn7B16>();
    assert_pyro_traits::<GTripleNestedListGroupU32W32>();
    assert_pyro_traits::<GTripleNestedListGroupU32W16>();
    assert_pyro_traits::<GBoolInner32>();
    assert_pyro_traits::<GBoolInner16>();
    assert_pyro_traits::<GListGroupBool32>();
    assert_pyro_traits::<GListGroupBool16>();
    assert_pyro_traits::<GTwoStrInner32>();
    assert_pyro_traits::<GTwoStrInner16>();
    assert_pyro_traits::<GListGroupTwoStr32>();
    assert_pyro_traits::<GListGroupTwoStr16>();
}
