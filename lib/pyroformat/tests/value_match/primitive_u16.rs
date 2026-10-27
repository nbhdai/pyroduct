//! Struct-vs-value match for every `PrimitiveDataType` variant.
//!
//! `src/wire/tests/primitive_u16.rs` already pins each case's `expected_buf`/
//! `expected_inodes` against the value/wire path — this format's golden
//! truth (`wire/mod.rs`'s `fixtures` feature doc comment). This module
//! reuses those same consts (imported, not re-typed) to pin a
//! `Layout, Writer, Reader`-derived one-field struct's inodes, writer output, and
//! read-back against them.
//!
//! `assert_roundtrip` checks `T::INODES` against the golden inodes, encodes
//! the data and compares the bytes against the golden buffer, then reads
//! the encoded buffer back and realizes it, asserting equality with the
//! original data.

use half::f16;

use pyroformat::value::Time;
use pyroformat::wire::tests::primitive_u16::*;

use crate::support::read_back::assert_roundtrip;
use pyroformat::{Layout, Reader, Writer};

macro_rules! value_match_primitive_test {
    ($test_name:ident, $struct_ty:ident, $field:expr, $buf_const:expr, $inodes_const:expr) => {
        #[test]
        fn $test_name() {
            let data = $struct_ty { a: $field };
            assert_roundtrip(&data, &$buf_const, &$inodes_const);
        }
    };
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PBool {
    a: bool,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PI8 {
    a: i8,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PI16 {
    a: i16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PI32 {
    a: i32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PI64 {
    a: i64,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PU8 {
    a: u8,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PU16 {
    a: u16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PU32 {
    a: u32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PU64 {
    a: u64,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PF16 {
    a: f16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PF32 {
    a: f32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PF64 {
    a: f64,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PTimestamp {
    a: Time,
}

// ── Basic values ─────────────────────────────────────────────────────────

value_match_primitive_test!(value_match_basic_bool, PBool, true, _BOOL, _BOOL_INODES);
value_match_primitive_test!(value_match_basic_i8, PI8, -100i8, _I8, _I8_INODES);
value_match_primitive_test!(value_match_basic_i16, PI16, -12345i16, _I16, _I16_INODES);
value_match_primitive_test!(value_match_basic_i32, PI32, -123456i32, _I32, _I32_INODES);
value_match_primitive_test!(
    value_match_basic_i64,
    PI64,
    -1234567890123i64,
    _I64,
    _I64_INODES
);
value_match_primitive_test!(value_match_basic_u8, PU8, 200u8, _U8, _U8_INODES);
value_match_primitive_test!(value_match_basic_u16, PU16, 54321u16, _U16, _U16_INODES);
value_match_primitive_test!(value_match_basic_u32, PU32, 123456u32, _U32, _U32_INODES);
value_match_primitive_test!(
    value_match_basic_u64,
    PU64,
    12345678901234567890u64,
    _U64,
    _U64_INODES
);
value_match_primitive_test!(
    value_match_basic_f16,
    PF16,
    f16::from_f32(1.5),
    _F16,
    _F16_INODES
);
value_match_primitive_test!(value_match_basic_f32, PF32, 3.14f32, _F32, _F32_INODES);
value_match_primitive_test!(
    value_match_basic_f64,
    PF64,
    2.718281828f64,
    _F64,
    _F64_INODES
);

// ── Edge-case values ─────────────────────────────────────────────────────

value_match_primitive_test!(
    value_match_bool_false,
    PBool,
    false,
    _BOOL_FALSE,
    _BOOL_FALSE_INODES
);

value_match_primitive_test!(value_match_i8_min, PI8, i8::MIN, _I8_MIN, _I8_MIN_INODES);
value_match_primitive_test!(value_match_i8_max, PI8, i8::MAX, _I8_MAX, _I8_MAX_INODES);

value_match_primitive_test!(
    value_match_i16_min,
    PI16,
    i16::MIN,
    _I16_MIN,
    _I16_MIN_INODES
);
value_match_primitive_test!(
    value_match_i16_max,
    PI16,
    i16::MAX,
    _I16_MAX,
    _I16_MAX_INODES
);

value_match_primitive_test!(
    value_match_i32_min,
    PI32,
    i32::MIN,
    _I32_MIN,
    _I32_MIN_INODES
);
value_match_primitive_test!(
    value_match_i32_max,
    PI32,
    i32::MAX,
    _I32_MAX,
    _I32_MAX_INODES
);

value_match_primitive_test!(
    value_match_i64_min,
    PI64,
    i64::MIN,
    _I64_MIN,
    _I64_MIN_INODES
);
value_match_primitive_test!(
    value_match_i64_max,
    PI64,
    i64::MAX,
    _I64_MAX,
    _I64_MAX_INODES
);

value_match_primitive_test!(value_match_u8_min, PU8, u8::MIN, _U8_MIN, _U8_MIN_INODES);
value_match_primitive_test!(value_match_u8_max, PU8, u8::MAX, _U8_MAX, _U8_MAX_INODES);

value_match_primitive_test!(
    value_match_u16_min,
    PU16,
    u16::MIN,
    _U16_MIN,
    _U16_MIN_INODES
);
value_match_primitive_test!(
    value_match_u16_max,
    PU16,
    u16::MAX,
    _U16_MAX,
    _U16_MAX_INODES
);

value_match_primitive_test!(
    value_match_u32_min,
    PU32,
    u32::MIN,
    _U32_MIN,
    _U32_MIN_INODES
);
value_match_primitive_test!(
    value_match_u32_max,
    PU32,
    u32::MAX,
    _U32_MAX,
    _U32_MAX_INODES
);

value_match_primitive_test!(
    value_match_u64_min,
    PU64,
    u64::MIN,
    _U64_MIN,
    _U64_MIN_INODES
);
value_match_primitive_test!(
    value_match_u64_max,
    PU64,
    u64::MAX,
    _U64_MAX,
    _U64_MAX_INODES
);

value_match_primitive_test!(
    value_match_f16_min,
    PF16,
    f16::MIN,
    _F16_MIN,
    _F16_MIN_INODES
);
value_match_primitive_test!(
    value_match_f16_max,
    PF16,
    f16::MAX,
    _F16_MAX,
    _F16_MAX_INODES
);
value_match_primitive_test!(
    value_match_f16_subnormal,
    PF16,
    f16::from_bits(1),
    _F16_SUBNORMAL,
    _F16_SUBNORMAL_INODES
);
value_match_primitive_test!(
    value_match_f16_neg_zero,
    PF16,
    f16::from_bits(0x8000),
    _F16_NEG_ZERO,
    _F16_NEG_ZERO_INODES
);
value_match_primitive_test!(
    value_match_f16_infinity,
    PF16,
    f16::INFINITY,
    _F16_INFINITY,
    _F16_INFINITY_INODES
);

value_match_primitive_test!(
    value_match_f32_min,
    PF32,
    f32::MIN,
    _F32_MIN,
    _F32_MIN_INODES
);
value_match_primitive_test!(
    value_match_f32_max,
    PF32,
    f32::MAX,
    _F32_MAX,
    _F32_MAX_INODES
);
value_match_primitive_test!(
    value_match_f32_subnormal,
    PF32,
    f32::from_bits(1),
    _F32_SUBNORMAL,
    _F32_SUBNORMAL_INODES
);
value_match_primitive_test!(
    value_match_f32_neg_zero,
    PF32,
    -0.0f32,
    _F32_NEG_ZERO,
    _F32_NEG_ZERO_INODES
);
value_match_primitive_test!(
    value_match_f32_infinity,
    PF32,
    f32::INFINITY,
    _F32_INFINITY,
    _F32_INFINITY_INODES
);

value_match_primitive_test!(
    value_match_f64_min,
    PF64,
    f64::MIN,
    _F64_MIN,
    _F64_MIN_INODES
);
value_match_primitive_test!(
    value_match_f64_max,
    PF64,
    f64::MAX,
    _F64_MAX,
    _F64_MAX_INODES
);
value_match_primitive_test!(
    value_match_f64_subnormal,
    PF64,
    f64::from_bits(1),
    _F64_SUBNORMAL,
    _F64_SUBNORMAL_INODES
);
value_match_primitive_test!(
    value_match_f64_neg_zero,
    PF64,
    -0.0f64,
    _F64_NEG_ZERO,
    _F64_NEG_ZERO_INODES
);
value_match_primitive_test!(
    value_match_f64_infinity,
    PF64,
    f64::INFINITY,
    _F64_INFINITY,
    _F64_INFINITY_INODES
);

value_match_primitive_test!(
    value_match_basic_timestamp,
    PTimestamp,
    Time(999_999_999),
    _TIMESTAMP,
    _TIMESTAMP_INODES
);

value_match_primitive_test!(
    value_match_negative_timestamp,
    PTimestamp,
    Time(-1_000_000_000),
    _TIMESTAMP_NEGATIVE,
    _TIMESTAMP_NEGATIVE_INODES
);
