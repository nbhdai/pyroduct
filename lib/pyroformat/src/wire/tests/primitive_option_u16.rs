#[cfg(test)]
use half::f16;
#[cfg(test)]
use pyro_schema::PrimitiveDataType;
#[cfg(test)]
use pyro_schema::human::{PyroField, PyroSchema, PyroType};

#[cfg(test)]
use crate::{PyroValue, value::Time, wire::tests::support::run_list_wrapped_roundtrip_u16};

use pyro_schema::{Inode, KindMeta, WireKind};

/// Every case below shares this `KindMeta` shape — a lone `Option` scalar
/// (`nesting: 0`) always lands at `data_offset: 0` with no group/nullable
/// bits set — only `kind` differs, same rationale as `primitive.rs`'s
/// `basic_inode!`.
macro_rules! option_inode {
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

/// One standalone `Option<T>` field (nesting=0) per primitive kind, each
/// tested both `Some` (present) and `None` (null), covering every kind
/// `primitive.rs` covers for the required variant. A lone `Option` scalar
/// with `nesting == 0` meets no condition in wiki/04-layout-and-alignment
/// 4.3.5's table, so it needs no trailing alignment padding: the buffer is
/// exactly the value's own bytes (zeroed when null) followed by one
/// validity bitvec byte (4.6 — a single presence bit needs `ceil(1/8) == 1`
/// byte), bit 0 set iff the value is present.
///
/// Each case's `Some`/`None` expected buffers are named `pub const`s
/// declared right above the macro invocation that uses them (`OPTION_*_SOME`
/// / `OPTION_*_NONE`) rather than being built dynamically in the macro body
/// — the `Some` buffer is the value's independently-computed LE encoding
/// with one trailing `1` byte appended, and the `None` buffer is the same
/// length, zeroed, with a trailing `0` byte.
macro_rules! option_roundtrip_test {
    (
        $some_name:ident,
        $none_name:ident,
        $wire_kind:ident,
        $schema_type:expr,
        $none_ty:ty,
        $inner_value:expr,
        $some_buf_const:ident,
        $none_buf_const:ident,
        $inodes_const:ident
    ) => {
        #[cfg_attr(test, tracing_test::traced_test)]
        #[test]
        fn $some_name() {
            let schema = PyroSchema {
                doc: String::new(),
                fields: vec![PyroField {
                    name: "a".to_string(),
                    doc: String::new(),
                    data_type: $schema_type,
                }],
            };

            let data = PyroValue::from(Some($inner_value));

            // A lone `Option` scalar (nesting=0) always encodes as its own
            // value bytes (that element's alignment) plus one trailing
            // validity-bitvec byte — never more, regardless of kind.
            assert_eq!($some_buf_const.len(), $inodes_const[0].alignment() + 1);

            run_list_wrapped_roundtrip_u16(schema, data, &$some_buf_const, &$inodes_const);
        }

        #[cfg_attr(test, tracing_test::traced_test)]
        #[test]
        fn $none_name() {
            let schema = PyroSchema {
                doc: String::new(),
                fields: vec![PyroField {
                    name: "a".to_string(),
                    doc: String::new(),
                    data_type: $schema_type,
                }],
            };

            let data = PyroValue::from(None::<$none_ty>);

            // Same invariant as the `Some` case above: alignment + 1 byte,
            // just with the value bytes zeroed instead of populated.
            assert_eq!($none_buf_const.len(), $inodes_const[0].alignment() + 1);

            run_list_wrapped_roundtrip_u16(schema, data, &$none_buf_const, &$inodes_const);
        }
    };
}

pub const OPTION_BOOL_SOME: [u8; 2] = [1, 1];
pub const OPTION_BOOL_NONE: [u8; 2] = [0, 0];
pub const OPTION_BOOL_INODES: [Inode; 1] = [option_inode!(OptionBool)];
option_roundtrip_test!(
    test_value_writer_option_bool_some,
    test_value_writer_option_bool_none,
    OptionBool,
    PyroType::OptionPrimitiveScalar {
        primitive: PrimitiveDataType::Bool
    },
    bool,
    true,
    OPTION_BOOL_SOME,
    OPTION_BOOL_NONE,
    OPTION_BOOL_INODES
);

pub const OPTION_I8_SOME: [u8; 2] = [156, 1];
pub const OPTION_I8_NONE: [u8; 2] = [0, 0];
pub const OPTION_I8_INODES: [Inode; 1] = [option_inode!(OptionI8)];
option_roundtrip_test!(
    test_value_writer_option_i8_some,
    test_value_writer_option_i8_none,
    OptionI8,
    PyroType::OptionPrimitiveScalar {
        primitive: PrimitiveDataType::I8
    },
    i8,
    -100i8,
    OPTION_I8_SOME,
    OPTION_I8_NONE,
    OPTION_I8_INODES
);

pub const OPTION_I16_SOME: [u8; 3] = [199, 207, 1];
pub const OPTION_I16_NONE: [u8; 3] = [0, 0, 0];
pub const OPTION_I16_INODES: [Inode; 1] = [option_inode!(OptionI16)];
option_roundtrip_test!(
    test_value_writer_option_i16_some,
    test_value_writer_option_i16_none,
    OptionI16,
    PyroType::OptionPrimitiveScalar {
        primitive: PrimitiveDataType::I16
    },
    i16,
    -12345i16,
    OPTION_I16_SOME,
    OPTION_I16_NONE,
    OPTION_I16_INODES
);

pub const OPTION_I32_SOME: [u8; 5] = [192, 29, 254, 255, 1];
pub const OPTION_I32_NONE: [u8; 5] = [0, 0, 0, 0, 0];
pub const OPTION_I32_INODES: [Inode; 1] = [option_inode!(OptionI32)];
option_roundtrip_test!(
    test_value_writer_option_i32_some,
    test_value_writer_option_i32_none,
    OptionI32,
    PyroType::OptionPrimitiveScalar {
        primitive: PrimitiveDataType::I32
    },
    i32,
    -123456i32,
    OPTION_I32_SOME,
    OPTION_I32_NONE,
    OPTION_I32_INODES
);

pub const OPTION_I64_SOME: [u8; 9] = [53, 251, 4, 142, 224, 254, 255, 255, 1];
pub const OPTION_I64_NONE: [u8; 9] = [0, 0, 0, 0, 0, 0, 0, 0, 0];
pub const OPTION_I64_INODES: [Inode; 1] = [option_inode!(OptionI64)];
option_roundtrip_test!(
    test_value_writer_option_i64_some,
    test_value_writer_option_i64_none,
    OptionI64,
    PyroType::OptionPrimitiveScalar {
        primitive: PrimitiveDataType::I64
    },
    i64,
    -1234567890123i64,
    OPTION_I64_SOME,
    OPTION_I64_NONE,
    OPTION_I64_INODES
);

pub const OPTION_U8_SOME: [u8; 2] = [5, 1];
pub const OPTION_U8_NONE: [u8; 2] = [0, 0];
pub const OPTION_U8_INODES: [Inode; 1] = [option_inode!(OptionU8)];
option_roundtrip_test!(
    test_value_writer_option_u8_some,
    test_value_writer_option_u8_none,
    OptionU8,
    PyroType::OptionPrimitiveScalar {
        primitive: PrimitiveDataType::U8
    },
    u8,
    5u8,
    OPTION_U8_SOME,
    OPTION_U8_NONE,
    OPTION_U8_INODES
);

pub const OPTION_U16_SOME: [u8; 3] = [49, 212, 1];
pub const OPTION_U16_NONE: [u8; 3] = [0, 0, 0];
pub const OPTION_U16_INODES: [Inode; 1] = [option_inode!(OptionU16)];
option_roundtrip_test!(
    test_value_writer_option_u16_some,
    test_value_writer_option_u16_none,
    OptionU16,
    PyroType::OptionPrimitiveScalar {
        primitive: PrimitiveDataType::U16
    },
    u16,
    54321u16,
    OPTION_U16_SOME,
    OPTION_U16_NONE,
    OPTION_U16_INODES
);

pub const OPTION_U32_SOME: [u8; 5] = [64, 226, 1, 0, 1];
pub const OPTION_U32_NONE: [u8; 5] = [0, 0, 0, 0, 0];
pub const OPTION_U32_INODES: [Inode; 1] = [option_inode!(OptionU32)];
option_roundtrip_test!(
    test_value_writer_option_u32_some,
    test_value_writer_option_u32_none,
    OptionU32,
    PyroType::OptionPrimitiveScalar {
        primitive: PrimitiveDataType::U32
    },
    u32,
    123456u32,
    OPTION_U32_SOME,
    OPTION_U32_NONE,
    OPTION_U32_INODES
);

pub const OPTION_U64_SOME: [u8; 9] = [210, 10, 31, 235, 140, 169, 84, 171, 1];
pub const OPTION_U64_NONE: [u8; 9] = [0, 0, 0, 0, 0, 0, 0, 0, 0];
pub const OPTION_U64_INODES: [Inode; 1] = [option_inode!(OptionU64)];
option_roundtrip_test!(
    test_value_writer_option_u64_some,
    test_value_writer_option_u64_none,
    OptionU64,
    PyroType::OptionPrimitiveScalar {
        primitive: PrimitiveDataType::U64
    },
    u64,
    12345678901234567890u64,
    OPTION_U64_SOME,
    OPTION_U64_NONE,
    OPTION_U64_INODES
);

pub const OPTION_F16_SOME: [u8; 3] = [0, 62, 1];
pub const OPTION_F16_NONE: [u8; 3] = [0, 0, 0];
pub const OPTION_F16_INODES: [Inode; 1] = [option_inode!(OptionF16)];
option_roundtrip_test!(
    test_value_writer_option_f16_some,
    test_value_writer_option_f16_none,
    OptionF16,
    PyroType::OptionPrimitiveScalar {
        primitive: PrimitiveDataType::F16
    },
    f16,
    f16::from_f32(1.5),
    OPTION_F16_SOME,
    OPTION_F16_NONE,
    OPTION_F16_INODES
);

pub const OPTION_F32_SOME: [u8; 5] = [195, 245, 72, 64, 1];
pub const OPTION_F32_NONE: [u8; 5] = [0, 0, 0, 0, 0];
pub const OPTION_F32_INODES: [Inode; 1] = [option_inode!(OptionF32)];
option_roundtrip_test!(
    test_value_writer_option_f32_some,
    test_value_writer_option_f32_none,
    OptionF32,
    PyroType::OptionPrimitiveScalar {
        primitive: PrimitiveDataType::F32
    },
    f32,
    3.14f32,
    OPTION_F32_SOME,
    OPTION_F32_NONE,
    OPTION_F32_INODES
);

pub const OPTION_F64_SOME: [u8; 9] = [155, 145, 4, 139, 10, 191, 5, 64, 1];
pub const OPTION_F64_NONE: [u8; 9] = [0, 0, 0, 0, 0, 0, 0, 0, 0];
pub const OPTION_F64_INODES: [Inode; 1] = [option_inode!(OptionF64)];
option_roundtrip_test!(
    test_value_writer_option_f64_some,
    test_value_writer_option_f64_none,
    OptionF64,
    PyroType::OptionPrimitiveScalar {
        primitive: PrimitiveDataType::F64
    },
    f64,
    2.718281828f64,
    OPTION_F64_SOME,
    OPTION_F64_NONE,
    OPTION_F64_INODES
);

pub const OPTION_TIMESTAMP_SOME: [u8; 17] =
    [255, 201, 154, 59, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];
pub const OPTION_TIMESTAMP_NONE: [u8; 17] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
pub const OPTION_TIMESTAMP_INODES: [Inode; 1] = [option_inode!(OptionTimestamp)];
option_roundtrip_test!(
    test_value_writer_option_timestamp_some,
    test_value_writer_option_timestamp_none,
    OptionTimestamp,
    PyroType::OptionPrimitiveScalar {
        primitive: PrimitiveDataType::Timestamp,
    },
    Time,
    Time(999_999_999),
    OPTION_TIMESTAMP_SOME,
    OPTION_TIMESTAMP_NONE,
    OPTION_TIMESTAMP_INODES
);
