#[cfg(test)]
use half::f16;
#[cfg(test)]
use pyro_schema::PrimitiveDataType;
#[cfg(test)]
use pyro_schema::human::{PyroField, PyroSchema, PyroType};

#[cfg(test)]
use crate::{PyroValue, wire::tests::support::run_list_wrapped_roundtrip_u16};

use pyro_schema::{Inode, KindMeta, WireKind};

/// Every case below shares this `KindMeta` shape — a lone `PrimitiveList`
/// element (`nesting: 1`) always lands at `data_offset: 0` with no
/// group/nullable bits set — only `kind` differs, same rationale as
/// `primitive.rs`'s `basic_inode!`.
macro_rules! list_inode {
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

macro_rules! primitive_list_roundtrip_test {
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
                        element: Box::new(PyroType::PrimitiveScalar {
                            primitive: PrimitiveDataType::$pdt,
                        }),
                    },
                }],
            };

            let data = PyroValue::from($value);

            run_list_wrapped_roundtrip_u16(schema, data, &$buf_const, &$inodes_const);
        }
    };
}

pub const BASIC_BOOL_LIST: [u8; 5] = [4, 0, 3, 0, 5];
pub const BASIC_BOOL_LIST_INODES: [Inode; 1] = [list_inode!(Bool)];
primitive_list_roundtrip_test!(
    test_value_writer_basic_bool_list,
    Bool,
    Bool,
    vec![true, false, true],
    BASIC_BOOL_LIST,
    BASIC_BOOL_LIST_INODES
);

pub const BASIC_I8_LIST: [u8; 7] = [4, 0, 3, 0, 255, 0, 1];
pub const BASIC_I8_LIST_INODES: [Inode; 1] = [list_inode!(I8)];
primitive_list_roundtrip_test!(
    test_value_writer_basic_i8_list,
    I8,
    I8,
    vec![-1i8, 0, 1],
    BASIC_I8_LIST,
    BASIC_I8_LIST_INODES
);

pub const BASIC_I16_LIST: [u8; 10] = [4, 0, 3, 0, 255, 255, 0, 0, 1, 0];
pub const BASIC_I16_LIST_INODES: [Inode; 1] = [list_inode!(I16)];
primitive_list_roundtrip_test!(
    test_value_writer_basic_i16_list,
    I16,
    I16,
    vec![-1i16, 0, 1],
    BASIC_I16_LIST,
    BASIC_I16_LIST_INODES
);

pub const BASIC_I32_LIST: [u8; 16] = [4, 0, 3, 0, 255, 255, 255, 255, 0, 0, 0, 0, 1, 0, 0, 0];
pub const BASIC_I32_LIST_INODES: [Inode; 1] = [list_inode!(I32)];
primitive_list_roundtrip_test!(
    test_value_writer_basic_i32_list,
    I32,
    I32,
    vec![-1i32, 0, 1],
    BASIC_I32_LIST,
    BASIC_I32_LIST_INODES
);

pub const BASIC_I64_LIST: [u8; 32] = [
    8, 0, 3, 0, 0, 0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0,
    0, 0, 0, 0, 0, 0,
];
pub const BASIC_I64_LIST_INODES: [Inode; 1] = [list_inode!(I64)];
primitive_list_roundtrip_test!(
    test_value_writer_basic_i64_list,
    I64,
    I64,
    vec![-1i64, 0, 1],
    BASIC_I64_LIST,
    BASIC_I64_LIST_INODES
);

pub const BASIC_U8_LIST: [u8; 7] = [4, 0, 3, 0, 1, 2, 3];
pub const BASIC_U8_LIST_INODES: [Inode; 1] = [list_inode!(U8)];
primitive_list_roundtrip_test!(
    test_value_writer_basic_u8_list,
    U8,
    U8,
    vec![1u8, 2, 3],
    BASIC_U8_LIST,
    BASIC_U8_LIST_INODES
);

pub const BASIC_U16_LIST: [u8; 10] = [4, 0, 3, 0, 1, 0, 2, 0, 3, 0];
pub const BASIC_U16_LIST_INODES: [Inode; 1] = [list_inode!(U16)];
primitive_list_roundtrip_test!(
    test_value_writer_basic_u16_list,
    U16,
    U16,
    vec![1u16, 2, 3],
    BASIC_U16_LIST,
    BASIC_U16_LIST_INODES
);

pub const BASIC_U32_LIST: [u8; 16] = [4, 0, 3, 0, 1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0];
pub const BASIC_U32_LIST_INODES: [Inode; 1] = [list_inode!(U32)];
primitive_list_roundtrip_test!(
    test_value_writer_basic_u32_list,
    U32,
    U32,
    vec![1u32, 2, 3],
    BASIC_U32_LIST,
    BASIC_U32_LIST_INODES
);

pub const BASIC_U64_LIST: [u8; 32] = [
    8, 0, 3, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0,
];
pub const BASIC_U64_LIST_INODES: [Inode; 1] = [list_inode!(U64)];
primitive_list_roundtrip_test!(
    test_value_writer_basic_u64_list,
    U64,
    U64,
    vec![1u64, 2, 3],
    BASIC_U64_LIST,
    BASIC_U64_LIST_INODES
);

pub const BASIC_F16_LIST: [u8; 10] = [4, 0, 3, 0, 0, 60, 0, 64, 0, 66];
pub const BASIC_F16_LIST_INODES: [Inode; 1] = [list_inode!(F16)];
primitive_list_roundtrip_test!(
    test_value_writer_basic_f16_list,
    F16,
    F16,
    vec![f16::from_f32(1.0), f16::from_f32(2.0), f16::from_f32(3.0)],
    BASIC_F16_LIST,
    BASIC_F16_LIST_INODES
);

pub const BASIC_F32_LIST: [u8; 16] = [4, 0, 3, 0, 0, 0, 128, 63, 0, 0, 0, 64, 0, 0, 64, 64];
pub const BASIC_F32_LIST_INODES: [Inode; 1] = [list_inode!(F32)];
primitive_list_roundtrip_test!(
    test_value_writer_basic_f32_list,
    F32,
    F32,
    vec![1.0f32, 2.0, 3.0],
    BASIC_F32_LIST,
    BASIC_F32_LIST_INODES
);

pub const BASIC_F64_LIST: [u8; 32] = [
    8, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 240, 63, 0, 0, 0, 0, 0, 0, 0, 64, 0, 0, 0, 0, 0, 0,
    8, 64,
];
pub const BASIC_F64_LIST_INODES: [Inode; 1] = [list_inode!(F64)];
primitive_list_roundtrip_test!(
    test_value_writer_basic_f64_list,
    F64,
    F64,
    vec![1.0f64, 2.0, 3.0],
    BASIC_F64_LIST,
    BASIC_F64_LIST_INODES
);
