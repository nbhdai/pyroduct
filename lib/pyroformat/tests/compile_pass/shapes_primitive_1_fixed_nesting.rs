//! Every derived struct shape from `tests/value_match/primitive_1_fixed_nesting_u32.rs`, minus
//! the golden-fixture plumbing: each must derive `Layout`, `Writer`
//! and `Reader`. Generated from the value_match file — keep in sync.

use half::f16;
use pyroformat::{Layout, Reader, Writer};

fn assert_pyro_traits<T: Layout + Writer + Reader>() {}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1FixedU8W32 {
    a: Vec<[u8; 2]>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1FixedU8W16 {
    a: Vec<[u8; 2]>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1FixedI8W32 {
    a: Vec<[i8; 2]>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1FixedI8W16 {
    a: Vec<[i8; 2]>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1FixedU16W32 {
    a: Vec<[u16; 2]>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1FixedU16W16 {
    a: Vec<[u16; 2]>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1FixedI16W32 {
    a: Vec<[i16; 2]>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1FixedI16W16 {
    a: Vec<[i16; 2]>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1FixedU32W32 {
    a: Vec<[u32; 2]>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1FixedU32W16 {
    a: Vec<[u32; 2]>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1FixedI32W32 {
    a: Vec<[i32; 2]>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1FixedI32W16 {
    a: Vec<[i32; 2]>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1FixedU64W32 {
    a: Vec<[u64; 2]>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1FixedU64W16 {
    a: Vec<[u64; 2]>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1FixedI64W32 {
    a: Vec<[i64; 2]>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1FixedI64W16 {
    a: Vec<[i64; 2]>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1FixedF16W32 {
    a: Vec<[f16; 2]>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1FixedF16W16 {
    a: Vec<[f16; 2]>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1FixedF32W32 {
    a: Vec<[f32; 2]>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1FixedF32W16 {
    a: Vec<[f32; 2]>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1FixedF64W32 {
    a: Vec<[f64; 2]>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1FixedF64W16 {
    a: Vec<[f64; 2]>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1FixedBool32 {
    a: Vec<[bool; 2]>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1FixedBool16 {
    a: Vec<[bool; 2]>,
}

fn main() {
    assert_pyro_traits::<PNested1FixedU8W32>();
    assert_pyro_traits::<PNested1FixedU8W16>();
    assert_pyro_traits::<PNested1FixedI8W32>();
    assert_pyro_traits::<PNested1FixedI8W16>();
    assert_pyro_traits::<PNested1FixedU16W32>();
    assert_pyro_traits::<PNested1FixedU16W16>();
    assert_pyro_traits::<PNested1FixedI16W32>();
    assert_pyro_traits::<PNested1FixedI16W16>();
    assert_pyro_traits::<PNested1FixedU32W32>();
    assert_pyro_traits::<PNested1FixedU32W16>();
    assert_pyro_traits::<PNested1FixedI32W32>();
    assert_pyro_traits::<PNested1FixedI32W16>();
    assert_pyro_traits::<PNested1FixedU64W32>();
    assert_pyro_traits::<PNested1FixedU64W16>();
    assert_pyro_traits::<PNested1FixedI64W32>();
    assert_pyro_traits::<PNested1FixedI64W16>();
    assert_pyro_traits::<PNested1FixedF16W32>();
    assert_pyro_traits::<PNested1FixedF16W16>();
    assert_pyro_traits::<PNested1FixedF32W32>();
    assert_pyro_traits::<PNested1FixedF32W16>();
    assert_pyro_traits::<PNested1FixedF64W32>();
    assert_pyro_traits::<PNested1FixedF64W16>();
    assert_pyro_traits::<PNested1FixedBool32>();
    assert_pyro_traits::<PNested1FixedBool16>();
}
