#[cfg(test)]
use half::f16;
#[cfg(test)]
use pyro_schema::PrimitiveDataType;
#[cfg(test)]
use pyro_schema::human::{PyroField, PyroSchema, PyroType};

#[cfg(test)]
use crate::{PyroValue, wire::tests::support::run_list_wrapped_roundtrip_u32};

use pyro_schema::{Inode, KindMeta, WireKind};

/// `Vec<Option<T>>` — a list whose *elements* are individually nullable, as
/// opposed to `primitive_option.rs`'s nullable scalar or
/// `primitive_slice.rs`'s list of required elements. There's no
/// `PrimitiveList`-of-`Option` shortcut in the schema type system
/// (`human::PyroType::PrimitiveList` only takes a bare `PrimitiveDataType`),
/// so this goes through the generic `List { element: OptionPrimitiveScalar }`
/// path instead, which lands on `WireKind::Option$pdt` with `nesting: 1` —
/// same inode shape `ListWriter::OptionU32` etc. exist for.
///
/// Per `src/wire/writers/option_list.rs`'s own doc comment, the wire shape
/// is: a 12-byte fixed part (`start: u32`, `count: u32` — present-value
/// count only, nulls excluded — `validity_len: u32` — total element count,
/// nulls included), then a dynamic payload of the present values packed
/// contiguously (nulls take no space there) followed by a bit-packed
/// (`Lsb0`) validity mask, one bit per element, bit set iff present.
///
/// Every test here uses the same 3-element shape, `[Some(v1), None,
/// Some(v2)]`, so `count == 2`, `validity_len == 3`, and the validity byte
/// is always `0b101 == 5` (bit 0 and bit 2 set, bit 1 clear).
///
/// `$expected_buf` assumes the fixed region is pre-allocated to exactly
/// `inode_data_len` (12 bytes here), the same assumption
/// `primitive_slice.rs` makes — so, like that file, these are expected to
/// fail for as long as `support.rs` pre-allocates from `encoded_len()`
/// instead.
/// Every case below shares this `KindMeta` shape — a lone `List<Option<T>>`
/// element (`nesting: 1`) always lands at `data_offset: 0` with no
/// group/nullable bits set — only `kind` differs, same rationale as
/// `primitive.rs`'s `basic_inode!`.
macro_rules! option_list_inode {
    ($wire_kind:ident) => {
        Inode {
            kind: WireKind::$wire_kind,
            kind_meta: KindMeta::Primitive {
                nesting: 1,
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

macro_rules! primitive_option_list_roundtrip_test {
    ($test_name:ident, $wire_kind:ident, $pdt:ident, $value:expr, $buf_const:ident, $inodes_const:ident) => {
        #[cfg_attr(test, tracing_test::traced_test)]
        #[test]
        fn $test_name() {
            let schema = PyroSchema {
                doc: String::new(),
                fields: vec![PyroField {
                    name: "a".to_string(),
                    doc: String::new(),
                    data_type: PyroType::List {
                        element: Box::new(PyroType::OptionPrimitiveScalar {
                            primitive: PrimitiveDataType::$pdt,
                        }),
                    },
                }],
            };

            let data = PyroValue::from($value);

            run_list_wrapped_roundtrip_u32(schema, data, &$buf_const, &$inodes_const);
        }
    };
}

pub const VALUE_WRITER_OPTION_BOOL_LIST: [u8; 14] = [12, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 1, 5];
pub const VALUE_WRITER_OPTION_BOOL_LIST_INODES: [Inode; 1] = [option_list_inode!(OptionBool)];
primitive_option_list_roundtrip_test!(
    test_value_writer_option_bool_list,
    OptionBool,
    Bool,
    vec![Some(true), None, Some(false)],
    VALUE_WRITER_OPTION_BOOL_LIST,
    VALUE_WRITER_OPTION_BOOL_LIST_INODES
);

pub const VALUE_WRITER_OPTION_I8_LIST: [u8; 15] = [12, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 1, 255, 5];
pub const VALUE_WRITER_OPTION_I8_LIST_INODES: [Inode; 1] = [option_list_inode!(OptionI8)];
primitive_option_list_roundtrip_test!(
    test_value_writer_option_i8_list,
    OptionI8,
    I8,
    vec![Some(1i8), None, Some(-1)],
    VALUE_WRITER_OPTION_I8_LIST,
    VALUE_WRITER_OPTION_I8_LIST_INODES
);

pub const VALUE_WRITER_OPTION_I16_LIST: [u8; 17] =
    [12, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 1, 0, 255, 255, 5];
pub const VALUE_WRITER_OPTION_I16_LIST_INODES: [Inode; 1] = [option_list_inode!(OptionI16)];
primitive_option_list_roundtrip_test!(
    test_value_writer_option_i16_list,
    OptionI16,
    I16,
    vec![Some(1i16), None, Some(-1)],
    VALUE_WRITER_OPTION_I16_LIST,
    VALUE_WRITER_OPTION_I16_LIST_INODES
);

pub const VALUE_WRITER_OPTION_I32_LIST: [u8; 21] = [
    12, 0, 0, 0, // offset
    2, 0, 0, 0, // count non-zero
    3, 0, 0, 0, // count total
    1, 0, 0, 0, // 1
    255, 255, 255, 255, // -1
    5,   // bitvec
];
pub const VALUE_WRITER_OPTION_I32_LIST_INODES: [Inode; 1] = [option_list_inode!(OptionI32)];
primitive_option_list_roundtrip_test!(
    test_value_writer_option_i32_list,
    OptionI32,
    I32,
    vec![Some(1i32), None, Some(-1)],
    VALUE_WRITER_OPTION_I32_LIST,
    VALUE_WRITER_OPTION_I32_LIST_INODES
);

pub const VALUE_WRITER_OPTION_I64_LIST: [u8; 33] = [
    16, 0, 0, 0, // offset
    2, 0, 0, 0, // count non-zero
    3, 0, 0, 0, // count total
    0, 0, 0, 0, // padding
    1, 0, 0, 0, 0, 0, 0, 0, // 1
    255, 255, 255, 255, 255, 255, 255, 255, // -1
    5,   // bitvec
];
pub const VALUE_WRITER_OPTION_I64_LIST_INODES: [Inode; 1] = [option_list_inode!(OptionI64)];
primitive_option_list_roundtrip_test!(
    test_value_writer_option_i64_list,
    OptionI64,
    I64,
    vec![Some(1i64), None, Some(-1)],
    VALUE_WRITER_OPTION_I64_LIST,
    VALUE_WRITER_OPTION_I64_LIST_INODES
);

pub const VALUE_WRITER_OPTION_U8_LIST: [u8; 15] = [
    12, 0, 0, 0, // offset
    2, 0, 0, 0, // count non-zero
    3, 0, 0, 0, // count total
    1, // 1
    3, // 3
    5, // mask
];
pub const VALUE_WRITER_OPTION_U8_LIST_INODES: [Inode; 1] = [option_list_inode!(OptionU8)];
primitive_option_list_roundtrip_test!(
    test_value_writer_option_u8_list,
    OptionU8,
    U8,
    vec![Some(1u8), None, Some(3)],
    VALUE_WRITER_OPTION_U8_LIST,
    VALUE_WRITER_OPTION_U8_LIST_INODES
);

pub const VALUE_WRITER_OPTION_U16_LIST: [u8; 17] = [
    12, 0, 0, 0, // Offset
    2, 0, 0, 0, // count non-zero
    3, 0, 0, 0, // count total
    1, 0, //
    3, 0, //
    5, //
];
pub const VALUE_WRITER_OPTION_U16_LIST_INODES: [Inode; 1] = [option_list_inode!(OptionU16)];
primitive_option_list_roundtrip_test!(
    test_value_writer_option_u16_list,
    OptionU16,
    U16,
    vec![Some(1u16), None, Some(3)],
    VALUE_WRITER_OPTION_U16_LIST,
    VALUE_WRITER_OPTION_U16_LIST_INODES
);

pub const VALUE_WRITER_OPTION_U32_LIST: [u8; 21] = [
    12, 0, 0, 0, //
    2, 0, 0, 0, //
    3, 0, 0, 0, //
    1, 0, 0, 0, //
    3, 0, 0, 0, //
    5, //
];
pub const VALUE_WRITER_OPTION_U32_LIST_INODES: [Inode; 1] = [option_list_inode!(OptionU32)];
primitive_option_list_roundtrip_test!(
    test_value_writer_option_u32_list,
    OptionU32,
    U32,
    vec![Some(1u32), None, Some(3)],
    VALUE_WRITER_OPTION_U32_LIST,
    VALUE_WRITER_OPTION_U32_LIST_INODES
);

pub const VALUE_WRITER_OPTION_U64_LIST: [u8; 33] = [
    16, 0, 0, 0, //
    2, 0, 0, 0, //
    3, 0, 0, 0, //
    0, 0, 0, 0, //
    1, 0, 0, 0, 0, 0, 0, 0, //
    3, 0, 0, 0, 0, 0, 0, 0, //
    5, //
];
pub const VALUE_WRITER_OPTION_U64_LIST_INODES: [Inode; 1] = [option_list_inode!(OptionU64)];
primitive_option_list_roundtrip_test!(
    test_value_writer_option_u64_list,
    OptionU64,
    U64,
    vec![Some(1u64), None, Some(3)],
    VALUE_WRITER_OPTION_U64_LIST,
    VALUE_WRITER_OPTION_U64_LIST_INODES
);

pub const VALUE_WRITER_OPTION_F16_LIST: [u8; 17] = [
    12, 0, 0, 0, //
    2, 0, 0, 0, //
    3, 0, 0, 0, //
    0, 60, //
    0, 66, //
    5,  //
];
pub const VALUE_WRITER_OPTION_F16_LIST_INODES: [Inode; 1] = [option_list_inode!(OptionF16)];
primitive_option_list_roundtrip_test!(
    test_value_writer_option_f16_list,
    OptionF16,
    F16,
    vec![Some(f16::from_f32(1.0)), None, Some(f16::from_f32(3.0))],
    VALUE_WRITER_OPTION_F16_LIST,
    VALUE_WRITER_OPTION_F16_LIST_INODES
);

pub const VALUE_WRITER_OPTION_F32_LIST: [u8; 21] = [
    12, 0, 0, 0, //
    2, 0, 0, 0, //
    3, 0, 0, 0, //
    0, 0, 128, 63, //
    0, 0, 64, 64, //
    5,  //
];
pub const VALUE_WRITER_OPTION_F32_LIST_INODES: [Inode; 1] = [option_list_inode!(OptionF32)];
primitive_option_list_roundtrip_test!(
    test_value_writer_option_f32_list,
    OptionF32,
    F32,
    vec![Some(1.0f32), None, Some(3.0)],
    VALUE_WRITER_OPTION_F32_LIST,
    VALUE_WRITER_OPTION_F32_LIST_INODES
);

pub const VALUE_WRITER_OPTION_F64_LIST: [u8; 33] = [
    16, 0, 0, 0, //
    2, 0, 0, 0, //
    3, 0, 0, 0, //
    0, 0, 0, 0, //
    0, 0, 0, 0, 0, 0, 240, 63, //
    0, 0, 0, 0, 0, 0, 8, 64, //
    5,  //
];
pub const VALUE_WRITER_OPTION_F64_LIST_INODES: [Inode; 1] = [option_list_inode!(OptionF64)];
primitive_option_list_roundtrip_test!(
    test_value_writer_option_f64_list,
    OptionF64,
    F64,
    vec![Some(1.0f64), None, Some(3.0)],
    VALUE_WRITER_OPTION_F64_LIST,
    VALUE_WRITER_OPTION_F64_LIST_INODES
);

pub const VALUE_WRITER_OPTION_U32_LIST_ALL_NONE: [u8; 13] = [
    12, 0, 0, 0, // offset
    0, 0, 0, 0, // present count
    3, 0, 0, 0, // total count
    0, // validity bitmask (all zero)
];
pub const VALUE_WRITER_OPTION_U32_LIST_ALL_NONE_INODES: [Inode; 1] =
    [option_list_inode!(OptionU32)];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_value_writer_option_u32_list_all_none() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::List {
                element: Box::new(PyroType::OptionPrimitiveScalar {
                    primitive: PrimitiveDataType::U32,
                }),
            },
        }],
    };

    let data: Vec<Option<u32>> = vec![None, None, None];
    let data = PyroValue::from(data);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &VALUE_WRITER_OPTION_U32_LIST_ALL_NONE,
        &VALUE_WRITER_OPTION_U32_LIST_ALL_NONE_INODES,
    );
}
