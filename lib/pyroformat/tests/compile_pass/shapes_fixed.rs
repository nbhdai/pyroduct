//! Every derived struct shape from `tests/value_match/fixed_u32.rs`, minus
//! the golden-fixture plumbing: each must derive `Layout`, `Writer`
//! and `Reader`. Generated from the value_match file — keep in sync.

use pyroformat::{Layout, Reader, Writer};

fn assert_pyro_traits<T: Layout + Writer + Reader>() {}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PFixedU32x4W32 {
    a: [u32; 4],
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PFixedU32x4W16 {
    a: [u32; 4],
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PFixedF64x2W32 {
    a: [f64; 2],
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PFixedF64x2W16 {
    a: [f64; 2],
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct POptionFixedF32x3W32 {
    a: Option<[f32; 3]>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct POptionFixedF32x3W16 {
    a: Option<[f32; 3]>,
}

fn main() {
    assert_pyro_traits::<PFixedU32x4W32>();
    assert_pyro_traits::<PFixedU32x4W16>();
    assert_pyro_traits::<PFixedF64x2W32>();
    assert_pyro_traits::<PFixedF64x2W16>();
    assert_pyro_traits::<POptionFixedF32x3W32>();
    assert_pyro_traits::<POptionFixedF32x3W16>();
}
