//! Every derived struct shape from `tests/value_match/primitive_5_fixed_nesting_u32.rs`, minus
//! the golden-fixture plumbing: each must derive `Layout`, `Writer`
//! and `Reader`. Generated from the value_match file — keep in sync.

use half::f16;
use pyroformat::{Layout, Reader, Writer};

fn assert_pyro_traits<T: Layout + Writer + Reader>() {}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested5FixedU8W32 {
    a: Vec<Vec<Vec<Vec<Vec<[u8; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested5FixedU8W16 {
    a: Vec<Vec<Vec<Vec<Vec<[u8; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested5FixedI8W32 {
    a: Vec<Vec<Vec<Vec<Vec<[i8; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested5FixedI8W16 {
    a: Vec<Vec<Vec<Vec<Vec<[i8; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested5FixedU16W32 {
    a: Vec<Vec<Vec<Vec<Vec<[u16; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested5FixedU16W16 {
    a: Vec<Vec<Vec<Vec<Vec<[u16; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested5FixedI16W32 {
    a: Vec<Vec<Vec<Vec<Vec<[i16; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested5FixedI16W16 {
    a: Vec<Vec<Vec<Vec<Vec<[i16; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested5FixedU32W32 {
    a: Vec<Vec<Vec<Vec<Vec<[u32; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested5FixedU32W16 {
    a: Vec<Vec<Vec<Vec<Vec<[u32; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested5FixedI32W32 {
    a: Vec<Vec<Vec<Vec<Vec<[i32; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested5FixedI32W16 {
    a: Vec<Vec<Vec<Vec<Vec<[i32; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested5FixedU64W32 {
    a: Vec<Vec<Vec<Vec<Vec<[u64; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested5FixedU64W16 {
    a: Vec<Vec<Vec<Vec<Vec<[u64; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested5FixedI64W32 {
    a: Vec<Vec<Vec<Vec<Vec<[i64; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested5FixedI64W16 {
    a: Vec<Vec<Vec<Vec<Vec<[i64; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested5FixedF16W32 {
    a: Vec<Vec<Vec<Vec<Vec<[f16; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested5FixedF16W16 {
    a: Vec<Vec<Vec<Vec<Vec<[f16; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested5FixedF32W32 {
    a: Vec<Vec<Vec<Vec<Vec<[f32; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested5FixedF32W16 {
    a: Vec<Vec<Vec<Vec<Vec<[f32; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested5FixedF64W32 {
    a: Vec<Vec<Vec<Vec<Vec<[f64; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested5FixedF64W16 {
    a: Vec<Vec<Vec<Vec<Vec<[f64; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested5FixedBool32 {
    a: Vec<Vec<Vec<Vec<Vec<[bool; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested5FixedBool16 {
    a: Vec<Vec<Vec<Vec<Vec<[bool; 2]>>>>>,
}

fn main() {
    assert_pyro_traits::<PNested5FixedU8W32>();
    assert_pyro_traits::<PNested5FixedU8W16>();
    assert_pyro_traits::<PNested5FixedI8W32>();
    assert_pyro_traits::<PNested5FixedI8W16>();
    assert_pyro_traits::<PNested5FixedU16W32>();
    assert_pyro_traits::<PNested5FixedU16W16>();
    assert_pyro_traits::<PNested5FixedI16W32>();
    assert_pyro_traits::<PNested5FixedI16W16>();
    assert_pyro_traits::<PNested5FixedU32W32>();
    assert_pyro_traits::<PNested5FixedU32W16>();
    assert_pyro_traits::<PNested5FixedI32W32>();
    assert_pyro_traits::<PNested5FixedI32W16>();
    assert_pyro_traits::<PNested5FixedU64W32>();
    assert_pyro_traits::<PNested5FixedU64W16>();
    assert_pyro_traits::<PNested5FixedI64W32>();
    assert_pyro_traits::<PNested5FixedI64W16>();
    assert_pyro_traits::<PNested5FixedF16W32>();
    assert_pyro_traits::<PNested5FixedF16W16>();
    assert_pyro_traits::<PNested5FixedF32W32>();
    assert_pyro_traits::<PNested5FixedF32W16>();
    assert_pyro_traits::<PNested5FixedF64W32>();
    assert_pyro_traits::<PNested5FixedF64W16>();
    assert_pyro_traits::<PNested5FixedBool32>();
    assert_pyro_traits::<PNested5FixedBool16>();
}
