//! Every derived struct shape from `tests/value_match/primitive_2_fixed_nesting_u32.rs`, minus
//! the golden-fixture plumbing: each must derive `Layout`, `Writer`
//! and `Reader`. Generated from the value_match file — keep in sync.

use half::f16;
use pyroformat::{Layout, Reader, Writer};

fn assert_pyro_traits<T: Layout + Writer + Reader>() {}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2FixedU8W32 {
    a: Vec<Vec<[u8; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested2FixedU8W16 {
    a: Vec<Vec<[u8; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2FixedI8W32 {
    a: Vec<Vec<[i8; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested2FixedI8W16 {
    a: Vec<Vec<[i8; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2FixedU16W32 {
    a: Vec<Vec<[u16; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested2FixedU16W16 {
    a: Vec<Vec<[u16; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2FixedI16W32 {
    a: Vec<Vec<[i16; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested2FixedI16W16 {
    a: Vec<Vec<[i16; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2FixedU32W32 {
    a: Vec<Vec<[u32; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested2FixedU32W16 {
    a: Vec<Vec<[u32; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2FixedI32W32 {
    a: Vec<Vec<[i32; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested2FixedI32W16 {
    a: Vec<Vec<[i32; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2FixedU64W32 {
    a: Vec<Vec<[u64; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested2FixedU64W16 {
    a: Vec<Vec<[u64; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2FixedI64W32 {
    a: Vec<Vec<[i64; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested2FixedI64W16 {
    a: Vec<Vec<[i64; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2FixedF16W32 {
    a: Vec<Vec<[f16; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested2FixedF16W16 {
    a: Vec<Vec<[f16; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2FixedF32W32 {
    a: Vec<Vec<[f32; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested2FixedF32W16 {
    a: Vec<Vec<[f32; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2FixedF64W32 {
    a: Vec<Vec<[f64; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested2FixedF64W16 {
    a: Vec<Vec<[f64; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2FixedBool32 {
    a: Vec<Vec<[bool; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested2FixedBool16 {
    a: Vec<Vec<[bool; 2]>>,
}

fn main() {
    assert_pyro_traits::<PNested2FixedU8W32>();
    assert_pyro_traits::<PNested2FixedU8W16>();
    assert_pyro_traits::<PNested2FixedI8W32>();
    assert_pyro_traits::<PNested2FixedI8W16>();
    assert_pyro_traits::<PNested2FixedU16W32>();
    assert_pyro_traits::<PNested2FixedU16W16>();
    assert_pyro_traits::<PNested2FixedI16W32>();
    assert_pyro_traits::<PNested2FixedI16W16>();
    assert_pyro_traits::<PNested2FixedU32W32>();
    assert_pyro_traits::<PNested2FixedU32W16>();
    assert_pyro_traits::<PNested2FixedI32W32>();
    assert_pyro_traits::<PNested2FixedI32W16>();
    assert_pyro_traits::<PNested2FixedU64W32>();
    assert_pyro_traits::<PNested2FixedU64W16>();
    assert_pyro_traits::<PNested2FixedI64W32>();
    assert_pyro_traits::<PNested2FixedI64W16>();
    assert_pyro_traits::<PNested2FixedF16W32>();
    assert_pyro_traits::<PNested2FixedF16W16>();
    assert_pyro_traits::<PNested2FixedF32W32>();
    assert_pyro_traits::<PNested2FixedF32W16>();
    assert_pyro_traits::<PNested2FixedF64W32>();
    assert_pyro_traits::<PNested2FixedF64W16>();
    assert_pyro_traits::<PNested2FixedBool32>();
    assert_pyro_traits::<PNested2FixedBool16>();
}
