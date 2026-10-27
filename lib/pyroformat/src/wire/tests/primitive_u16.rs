#[cfg(test)]
use half::f16;
#[cfg(test)]
use pyro_schema::{
    PrimitiveDataType,
    human::{PyroField, PyroSchema, PyroType},
};

#[cfg(test)]
use crate::{PyroValue, value::Time, wire::tests::support::run_list_wrapped_roundtrip_u16};

use pyro_schema::{Inode, KindMeta, WireKind};

/// One standalone scalar field (nesting=0, so it's the only thing in the
/// fixed region and always lands at `data_offset: 0`), round-tripped
/// through the real writer/reader and checked against independently
/// hand-computed little-endian bytes — same shape as
/// `test_value_writer_basic_u32` before this macro existed, just stamped
/// out once per [`PrimitiveDataType`] variant instead of copy-pasted.
///
/// Each case's `expected_buf`/`expected_inodes` are named `pub const`s
/// declared right above the macro invocation that uses them (`_BOOL`/
/// `_BOOL_INODES`, etc.) rather than inline literals, so they're real
/// importable items — `pyroformat::wire::tests::primitive_u16::_BOOL` — behind
/// the `fixtures` Cargo feature (see its doc comment in `Cargo.toml` and
/// `wire/mod.rs`), for `tests/` to assert against directly instead of
/// hand-copying the bytes a second time.
macro_rules! primitive_roundtrip_test {
    ($test_name:ident, $pdt:ident, $value:expr, $buf_const:ident, $inodes_const:ident) => {
        #[cfg_attr(test, tracing_test::traced_test)]
        #[test]
        fn $test_name() {
            let schema = PyroSchema {
                doc: String::new(),
                fields: vec![PyroField {
                    name: "a".to_string(),
                    doc: String::new(),
                    data_type: PyroType::PrimitiveScalar {
                        primitive: PrimitiveDataType::$pdt,
                    },
                }],
            };

            let data = PyroValue::from($value);

            run_list_wrapped_roundtrip_u16(schema, data, &$buf_const, &$inodes_const);
        }
    };
}

/// Every case below shares this `KindMeta` shape (a lone `nesting: 0`
/// scalar is the entire fixed region, so it always lands at `data_offset:
/// 0` with no group/nullable bits set) — only `kind` differs, so this
/// macro just fills that one slot in rather than repeating the other six
/// fields at every call site.
macro_rules! basic_inode {
    ($wire_kind:ident) => {
        Inode {
            kind: WireKind::$wire_kind,
            kind_meta: KindMeta::Primitive {
                nesting: 0,
                data_offset: 0,
                topology_offset: 0,
                fixed_size: 0,
                nullable_bit_index: 0,
                group: false,
                group_nest: false,
            },
        }
    };
}

pub const _BOOL: [u8; 1] = [1];
pub const _BOOL_INODES: [Inode; 1] = [basic_inode!(Bool)];
primitive_roundtrip_test!(
    test_value_writer_basic_bool,
    Bool,
    true,
    _BOOL,
    _BOOL_INODES
);

pub const _I8: [u8; 1] = [156];
pub const _I8_INODES: [Inode; 1] = [basic_inode!(I8)];
primitive_roundtrip_test!(test_value_writer_basic_i8, I8, -100i8, _I8, _I8_INODES);

pub const _I16: [u8; 2] = [199, 207];
pub const _I16_INODES: [Inode; 1] = [basic_inode!(I16)];
primitive_roundtrip_test!(
    test_value_writer_basic_i16,
    I16,
    -12345i16,
    _I16,
    _I16_INODES
);

pub const _I32: [u8; 4] = [192, 29, 254, 255];
pub const _I32_INODES: [Inode; 1] = [basic_inode!(I32)];
primitive_roundtrip_test!(
    test_value_writer_basic_i32,
    I32,
    -123456i32,
    _I32,
    _I32_INODES
);

pub const _I64: [u8; 8] = [53, 251, 4, 142, 224, 254, 255, 255];
pub const _I64_INODES: [Inode; 1] = [basic_inode!(I64)];
primitive_roundtrip_test!(
    test_value_writer_basic_i64,
    I64,
    -1234567890123i64,
    _I64,
    _I64_INODES
);

pub const _U8: [u8; 1] = [200];
pub const _U8_INODES: [Inode; 1] = [basic_inode!(U8)];
primitive_roundtrip_test!(test_value_writer_basic_u8, U8, 200u8, _U8, _U8_INODES);

pub const _U16: [u8; 2] = [49, 212];
pub const _U16_INODES: [Inode; 1] = [basic_inode!(U16)];
primitive_roundtrip_test!(
    test_value_writer_basic_u16,
    U16,
    54321u16,
    _U16,
    _U16_INODES
);

pub const _U32: [u8; 4] = [64, 226, 1, 0];
pub const _U32_INODES: [Inode; 1] = [basic_inode!(U32)];
primitive_roundtrip_test!(
    test_value_writer_basic_u32,
    U32,
    123456u32,
    _U32,
    _U32_INODES
);

pub const _U64: [u8; 8] = [210, 10, 31, 235, 140, 169, 84, 171];
pub const _U64_INODES: [Inode; 1] = [basic_inode!(U64)];
primitive_roundtrip_test!(
    test_value_writer_basic_u64,
    U64,
    12345678901234567890u64,
    _U64,
    _U64_INODES
);

pub const _F16: [u8; 2] = [0, 62];
pub const _F16_INODES: [Inode; 1] = [basic_inode!(F16)];
primitive_roundtrip_test!(
    test_value_writer_basic_f16,
    F16,
    f16::from_f32(1.5),
    _F16,
    _F16_INODES
);

pub const _F32: [u8; 4] = [195, 245, 72, 64];
pub const _F32_INODES: [Inode; 1] = [basic_inode!(F32)];
primitive_roundtrip_test!(test_value_writer_basic_f32, F32, 3.14f32, _F32, _F32_INODES);

pub const _F64: [u8; 8] = [155, 145, 4, 139, 10, 191, 5, 64];
pub const _F64_INODES: [Inode; 1] = [basic_inode!(F64)];
primitive_roundtrip_test!(
    test_value_writer_basic_f64,
    F64,
    2.718281828f64,
    _F64,
    _F64_INODES
);

// Edge-case values for every `PrimitiveDataType` variant, same shape as
// the basic cases above (`primitive_roundtrip_test!` + `basic_inode!`),
// just exercising the extremes of each type's representable range instead
// of an arbitrary mid-range value:
//   - every type gets its `MIN`/`MAX` (bool's "MIN" is `false`, since the
//     basic case above already covers `true`)
//   - the float types (`F16`/`F32`/`F64`) additionally get the smallest
//     positive subnormal (`from_bits(1)` — exponent all-zero, mantissa
//     `1`), negative zero (sign bit set, everything else zero — distinct
//     bits from `0.0` but `==` under IEEE 754, so the roundtrip's
//     `assert_eq!` only proves the *value* round-trips; `expected_buf`
//     below is what actually pins the sign bit), and positive infinity
//     (exponent all-one, mantissa zero) — none of which integers have an
//     analogue for
// `NaN` is deliberately excluded: `NaN != NaN`, so `assert_eq!(val,
// restored)` in `run_list_wrapped_roundtrip_u16` would fail even on a
// perfectly correct roundtrip.
//
// Every `expected_buf` below was hand-computed independently (Python's
// `struct.pack("<...", value)` / manual IEEE 754 bit-twiddling for `f16`,
// which has no native Python packer), not derived from the writer.

pub const _BOOL_FALSE: [u8; 1] = [0];
pub const _BOOL_FALSE_INODES: [Inode; 1] = [basic_inode!(Bool)];
primitive_roundtrip_test!(
    test_value_writer_bool_false,
    Bool,
    false,
    _BOOL_FALSE,
    _BOOL_FALSE_INODES
);

pub const _I8_MIN: [u8; 1] = [128];
pub const _I8_MIN_INODES: [Inode; 1] = [basic_inode!(I8)];
primitive_roundtrip_test!(
    test_value_writer_i8_min,
    I8,
    i8::MIN,
    _I8_MIN,
    _I8_MIN_INODES
);

pub const _I8_MAX: [u8; 1] = [127];
pub const _I8_MAX_INODES: [Inode; 1] = [basic_inode!(I8)];
primitive_roundtrip_test!(
    test_value_writer_i8_max,
    I8,
    i8::MAX,
    _I8_MAX,
    _I8_MAX_INODES
);

pub const _I16_MIN: [u8; 2] = [0, 128];
pub const _I16_MIN_INODES: [Inode; 1] = [basic_inode!(I16)];
primitive_roundtrip_test!(
    test_value_writer_i16_min,
    I16,
    i16::MIN,
    _I16_MIN,
    _I16_MIN_INODES
);

pub const _I16_MAX: [u8; 2] = [255, 127];
pub const _I16_MAX_INODES: [Inode; 1] = [basic_inode!(I16)];
primitive_roundtrip_test!(
    test_value_writer_i16_max,
    I16,
    i16::MAX,
    _I16_MAX,
    _I16_MAX_INODES
);

pub const _I32_MIN: [u8; 4] = [0, 0, 0, 128];
pub const _I32_MIN_INODES: [Inode; 1] = [basic_inode!(I32)];
primitive_roundtrip_test!(
    test_value_writer_i32_min,
    I32,
    i32::MIN,
    _I32_MIN,
    _I32_MIN_INODES
);

pub const _I32_MAX: [u8; 4] = [255, 255, 255, 127];
pub const _I32_MAX_INODES: [Inode; 1] = [basic_inode!(I32)];
primitive_roundtrip_test!(
    test_value_writer_i32_max,
    I32,
    i32::MAX,
    _I32_MAX,
    _I32_MAX_INODES
);

pub const _I64_MIN: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 128];
pub const _I64_MIN_INODES: [Inode; 1] = [basic_inode!(I64)];
primitive_roundtrip_test!(
    test_value_writer_i64_min,
    I64,
    i64::MIN,
    _I64_MIN,
    _I64_MIN_INODES
);

pub const _I64_MAX: [u8; 8] = [255, 255, 255, 255, 255, 255, 255, 127];
pub const _I64_MAX_INODES: [Inode; 1] = [basic_inode!(I64)];
primitive_roundtrip_test!(
    test_value_writer_i64_max,
    I64,
    i64::MAX,
    _I64_MAX,
    _I64_MAX_INODES
);

pub const _U8_MIN: [u8; 1] = [0];
pub const _U8_MIN_INODES: [Inode; 1] = [basic_inode!(U8)];
primitive_roundtrip_test!(
    test_value_writer_u8_min,
    U8,
    u8::MIN,
    _U8_MIN,
    _U8_MIN_INODES
);

pub const _U8_MAX: [u8; 1] = [255];
pub const _U8_MAX_INODES: [Inode; 1] = [basic_inode!(U8)];
primitive_roundtrip_test!(
    test_value_writer_u8_max,
    U8,
    u8::MAX,
    _U8_MAX,
    _U8_MAX_INODES
);

pub const _U16_MIN: [u8; 2] = [0, 0];
pub const _U16_MIN_INODES: [Inode; 1] = [basic_inode!(U16)];
primitive_roundtrip_test!(
    test_value_writer_u16_min,
    U16,
    u16::MIN,
    _U16_MIN,
    _U16_MIN_INODES
);

pub const _U16_MAX: [u8; 2] = [255, 255];
pub const _U16_MAX_INODES: [Inode; 1] = [basic_inode!(U16)];
primitive_roundtrip_test!(
    test_value_writer_u16_max,
    U16,
    u16::MAX,
    _U16_MAX,
    _U16_MAX_INODES
);

pub const _U32_MIN: [u8; 4] = [0, 0, 0, 0];
pub const _U32_MIN_INODES: [Inode; 1] = [basic_inode!(U32)];
primitive_roundtrip_test!(
    test_value_writer_u32_min,
    U32,
    u32::MIN,
    _U32_MIN,
    _U32_MIN_INODES
);

pub const _U32_MAX: [u8; 4] = [255, 255, 255, 255];
pub const _U32_MAX_INODES: [Inode; 1] = [basic_inode!(U32)];
primitive_roundtrip_test!(
    test_value_writer_u32_max,
    U32,
    u32::MAX,
    _U32_MAX,
    _U32_MAX_INODES
);

pub const _U64_MIN: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 0];
pub const _U64_MIN_INODES: [Inode; 1] = [basic_inode!(U64)];
primitive_roundtrip_test!(
    test_value_writer_u64_min,
    U64,
    u64::MIN,
    _U64_MIN,
    _U64_MIN_INODES
);

pub const _U64_MAX: [u8; 8] = [255, 255, 255, 255, 255, 255, 255, 255];
pub const _U64_MAX_INODES: [Inode; 1] = [basic_inode!(U64)];
primitive_roundtrip_test!(
    test_value_writer_u64_max,
    U64,
    u64::MAX,
    _U64_MAX,
    _U64_MAX_INODES
);

// f16 bit layout: 1 sign + 5 exponent + 10 mantissa, bias 15.
pub const _F16_MIN: [u8; 2] = [255, 251]; // bits 0xFBFF: sign=1, exp=11110, mantissa=all-1 => -65504
pub const _F16_MIN_INODES: [Inode; 1] = [basic_inode!(F16)];
primitive_roundtrip_test!(
    test_value_writer_f16_min,
    F16,
    f16::MIN,
    _F16_MIN,
    _F16_MIN_INODES
);

pub const _F16_MAX: [u8; 2] = [255, 123]; // bits 0x7BFF: sign=0, exp=11110, mantissa=all-1 => 65504
pub const _F16_MAX_INODES: [Inode; 1] = [basic_inode!(F16)];
primitive_roundtrip_test!(
    test_value_writer_f16_max,
    F16,
    f16::MAX,
    _F16_MAX,
    _F16_MAX_INODES
);

pub const _F16_SUBNORMAL: [u8; 2] = [1, 0]; // bits 0x0001: exp=all-0, mantissa=1 => smallest positive subnormal
pub const _F16_SUBNORMAL_INODES: [Inode; 1] = [basic_inode!(F16)];
primitive_roundtrip_test!(
    test_value_writer_f16_subnormal,
    F16,
    f16::from_bits(1),
    _F16_SUBNORMAL,
    _F16_SUBNORMAL_INODES
);

pub const _F16_NEG_ZERO: [u8; 2] = [0, 128]; // bits 0x8000: sign=1, everything else 0
pub const _F16_NEG_ZERO_INODES: [Inode; 1] = [basic_inode!(F16)];
primitive_roundtrip_test!(
    test_value_writer_f16_neg_zero,
    F16,
    f16::from_bits(0x8000),
    _F16_NEG_ZERO,
    _F16_NEG_ZERO_INODES
);

pub const _F16_INFINITY: [u8; 2] = [0, 124]; // bits 0x7C00: exp=all-1, mantissa=0
pub const _F16_INFINITY_INODES: [Inode; 1] = [basic_inode!(F16)];
primitive_roundtrip_test!(
    test_value_writer_f16_infinity,
    F16,
    f16::INFINITY,
    _F16_INFINITY,
    _F16_INFINITY_INODES
);

pub const _F32_MIN: [u8; 4] = [255, 255, 127, 255];
pub const _F32_MIN_INODES: [Inode; 1] = [basic_inode!(F32)];
primitive_roundtrip_test!(
    test_value_writer_f32_min,
    F32,
    f32::MIN,
    _F32_MIN,
    _F32_MIN_INODES
);

pub const _F32_MAX: [u8; 4] = [255, 255, 127, 127];
pub const _F32_MAX_INODES: [Inode; 1] = [basic_inode!(F32)];
primitive_roundtrip_test!(
    test_value_writer_f32_max,
    F32,
    f32::MAX,
    _F32_MAX,
    _F32_MAX_INODES
);

pub const _F32_SUBNORMAL: [u8; 4] = [1, 0, 0, 0]; // bits 1: smallest positive subnormal
pub const _F32_SUBNORMAL_INODES: [Inode; 1] = [basic_inode!(F32)];
primitive_roundtrip_test!(
    test_value_writer_f32_subnormal,
    F32,
    f32::from_bits(1),
    _F32_SUBNORMAL,
    _F32_SUBNORMAL_INODES
);

pub const _F32_NEG_ZERO: [u8; 4] = [0, 0, 0, 128]; // bits 0x80000000: sign bit only
pub const _F32_NEG_ZERO_INODES: [Inode; 1] = [basic_inode!(F32)];
primitive_roundtrip_test!(
    test_value_writer_f32_neg_zero,
    F32,
    -0.0f32,
    _F32_NEG_ZERO,
    _F32_NEG_ZERO_INODES
);

pub const _F32_INFINITY: [u8; 4] = [0, 0, 128, 127]; // bits 0x7F800000: exp=all-1, mantissa=0
pub const _F32_INFINITY_INODES: [Inode; 1] = [basic_inode!(F32)];
primitive_roundtrip_test!(
    test_value_writer_f32_infinity,
    F32,
    f32::INFINITY,
    _F32_INFINITY,
    _F32_INFINITY_INODES
);

pub const _F64_MIN: [u8; 8] = [255, 255, 255, 255, 255, 255, 239, 255];
pub const _F64_MIN_INODES: [Inode; 1] = [basic_inode!(F64)];
primitive_roundtrip_test!(
    test_value_writer_f64_min,
    F64,
    f64::MIN,
    _F64_MIN,
    _F64_MIN_INODES
);

pub const _F64_MAX: [u8; 8] = [255, 255, 255, 255, 255, 255, 239, 127];
pub const _F64_MAX_INODES: [Inode; 1] = [basic_inode!(F64)];
primitive_roundtrip_test!(
    test_value_writer_f64_max,
    F64,
    f64::MAX,
    _F64_MAX,
    _F64_MAX_INODES
);

pub const _F64_SUBNORMAL: [u8; 8] = [1, 0, 0, 0, 0, 0, 0, 0]; // bits 1: smallest positive subnormal
pub const _F64_SUBNORMAL_INODES: [Inode; 1] = [basic_inode!(F64)];
primitive_roundtrip_test!(
    test_value_writer_f64_subnormal,
    F64,
    f64::from_bits(1),
    _F64_SUBNORMAL,
    _F64_SUBNORMAL_INODES
);

pub const _F64_NEG_ZERO: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 128]; // bits 0x8000000000000000: sign bit only
pub const _F64_NEG_ZERO_INODES: [Inode; 1] = [basic_inode!(F64)];
primitive_roundtrip_test!(
    test_value_writer_f64_neg_zero,
    F64,
    -0.0f64,
    _F64_NEG_ZERO,
    _F64_NEG_ZERO_INODES
);

pub const _F64_INFINITY: [u8; 8] = [0, 0, 0, 0, 0, 0, 240, 127]; // bits 0x7FF0000000000000: exp=all-1, mantissa=0
pub const _F64_INFINITY_INODES: [Inode; 1] = [basic_inode!(F64)];
primitive_roundtrip_test!(
    test_value_writer_f64_infinity,
    F64,
    f64::INFINITY,
    _F64_INFINITY,
    _F64_INFINITY_INODES
);

// `Timestamp`: the same shape as the primitives above, spelled out by hand.
pub const _TIMESTAMP: [u8; 16] = [255, 201, 154, 59, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
pub const _TIMESTAMP_INODES: [Inode; 1] = [basic_inode!(Timestamp)];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_value_writer_basic_timestamp() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::PrimitiveScalar {
                primitive: PrimitiveDataType::Timestamp,
            },
        }],
    };

    let data = PyroValue::from(Time(999_999_999));

    run_list_wrapped_roundtrip_u16(schema, data, &_TIMESTAMP, &_TIMESTAMP_INODES);
}

pub const _TIMESTAMP_NEGATIVE: [u8; 16] = [
    0, 54, 101, 196, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
];
pub const _TIMESTAMP_NEGATIVE_INODES: [Inode; 1] = [basic_inode!(Timestamp)];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_value_writer_negative_timestamp() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::PrimitiveScalar {
                primitive: PrimitiveDataType::Timestamp,
            },
        }],
    };

    let data = PyroValue::from(Time(-1_000_000_000));

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &_TIMESTAMP_NEGATIVE,
        &_TIMESTAMP_NEGATIVE_INODES,
    );
}
