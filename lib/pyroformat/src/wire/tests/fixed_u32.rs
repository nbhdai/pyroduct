#[cfg(test)]
use pyro_schema::{
    PrimitiveDataType,
    human::{PyroField, PyroSchema, PyroType},
};

#[cfg(test)]
use crate::{
    PyroValue,
    value::{PrimitiveValueList, Time},
    wire::tests::support::run_list_wrapped_roundtrip_u32,
};

use pyro_schema::{Inode, KindMeta, WireKind};

// ---------------------------------------------------------------------------
// PrimitiveFixedList tests (fixed-size primitive arrays [T; N])
// ---------------------------------------------------------------------------

pub const FIXED_U32_4: [u8; 16] = [
    10, 0, 0, 0, // 10
    20, 0, 0, 0, // 20
    30, 0, 0, 0, // 30
    40, 0, 0, 0, // 40
];
pub const FIXED_U32_4_INODES: [Inode; 1] = [Inode {
    kind: WireKind::U32,
    kind_meta: KindMeta::Primitive {
        nesting: 0,
        data_offset: 0,
        topology_offset: 0,
        fixed_size: 4,
        nullable_bit_index: 0,
        group: false,
        group_nest: false,
    },
}];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_fixed_u32() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::PrimitiveFixedList {
                element: PrimitiveDataType::U32,
                size: 4,
            },
        }],
    };

    let data = PyroValue::PrimitiveList(PrimitiveValueList::U32(vec![10, 20, 30, 40].into()));

    run_list_wrapped_roundtrip_u32(schema, data, &FIXED_U32_4, &FIXED_U32_4_INODES);
}

pub const FIXED_F64_2: [u8; 16] = [
    0, 0, 0, 0, 0, 0, 240, 63, // 1.0f64 LE
    0, 0, 0, 0, 0, 0, 4, 64, // 2.5f64 LE
];
pub const FIXED_F64_2_INODES: [Inode; 1] = [Inode {
    kind: WireKind::F64,
    kind_meta: KindMeta::Primitive {
        nesting: 0,
        data_offset: 0,
        topology_offset: 0,
        fixed_size: 2,
        nullable_bit_index: 0,
        group: false,
        group_nest: false,
    },
}];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_fixed_f64() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::PrimitiveFixedList {
                element: PrimitiveDataType::F64,
                size: 2,
            },
        }],
    };

    let data = PyroValue::PrimitiveList(PrimitiveValueList::F64(vec![1.0, 2.5].into()));

    run_list_wrapped_roundtrip_u32(schema, data, &FIXED_F64_2, &FIXED_F64_2_INODES);
}

// ---------------------------------------------------------------------------
// OptionPrimitiveFixedList tests (Option<[T; N]>)
// ---------------------------------------------------------------------------

pub const OPTION_FIXED_F32_SOME: [u8; 13] = [
    0, 0, 128, 63, // 1.0f32
    0, 0, 0, 64, // 2.0f32
    0, 0, 64, 64, // 3.0f32
    1,  // validity byte: present
];
pub const OPTION_FIXED_F32_INODES: [Inode; 1] = [Inode {
    kind: WireKind::OptionF32,
    kind_meta: KindMeta::Primitive {
        nesting: 0,
        data_offset: 0,
        topology_offset: 0,
        fixed_size: 3,
        nullable_bit_index: 0,
        group: false,
        group_nest: false,
    },
}];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_option_fixed_f32_some() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::OptionPrimitiveFixedList {
                element: PrimitiveDataType::F32,
                size: 3,
            },
        }],
    };

    let data = PyroValue::PrimitiveList(PrimitiveValueList::F32(vec![1.0, 2.0, 3.0].into()));

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &OPTION_FIXED_F32_SOME,
        &OPTION_FIXED_F32_INODES,
    );
}

pub const OPTION_FIXED_F32_NONE: [u8; 13] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // validity byte: absent (null)
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_option_fixed_f32_none() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::OptionPrimitiveFixedList {
                element: PrimitiveDataType::F32,
                size: 3,
            },
        }],
    };

    let data = PyroValue::Null;

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &OPTION_FIXED_F32_NONE,
        &OPTION_FIXED_F32_INODES,
    );
}

// ---------------------------------------------------------------------------
// Timestamp arrays ([Time; N] / Option<[Time; N]>): 16 bytes per element
// ---------------------------------------------------------------------------

pub const FIXED_TIMESTAMP_2: [u8; 32] = [
    1, 202, 154, 59, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // Time(1_000_000_001)
    254, 107, 202, 136, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, // Time(-2_000_000_002)
];
pub const FIXED_TIMESTAMP_2_INODES: [Inode; 1] = [Inode {
    kind: WireKind::Timestamp,
    kind_meta: KindMeta::Primitive {
        nesting: 0,
        data_offset: 0,
        topology_offset: 0,
        fixed_size: 2,
        nullable_bit_index: 0,
        group: false,
        group_nest: false,
    },
}];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_fixed_timestamp() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::PrimitiveFixedList {
                element: PrimitiveDataType::Timestamp,
                size: 2,
            },
        }],
    };

    let data = PyroValue::from(vec![Time(1_000_000_001), Time(-2_000_000_002)]);

    run_list_wrapped_roundtrip_u32(schema, data, &FIXED_TIMESTAMP_2, &FIXED_TIMESTAMP_2_INODES);
}

pub const OPTION_FIXED_TIMESTAMP_SOME: [u8; 33] = [
    1, 202, 154, 59, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // Time(1_000_000_001)
    254, 107, 202, 136, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, // Time(-2_000_000_002)
    1,   // validity byte: present
];
pub const OPTION_FIXED_TIMESTAMP_INODES: [Inode; 1] = [Inode {
    kind: WireKind::OptionTimestamp,
    kind_meta: KindMeta::Primitive {
        nesting: 0,
        data_offset: 0,
        topology_offset: 0,
        fixed_size: 2,
        nullable_bit_index: 0,
        group: false,
        group_nest: false,
    },
}];

#[cfg(test)]
fn option_fixed_timestamp_schema() -> pyro_schema::human::PyroSchema {
    PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::OptionPrimitiveFixedList {
                element: PrimitiveDataType::Timestamp,
                size: 2,
            },
        }],
    }
}

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_option_fixed_timestamp_some() {
    let data = PyroValue::from(vec![Time(1_000_000_001), Time(-2_000_000_002)]);

    run_list_wrapped_roundtrip_u32(
        option_fixed_timestamp_schema(),
        data,
        &OPTION_FIXED_TIMESTAMP_SOME,
        &OPTION_FIXED_TIMESTAMP_INODES,
    );
}

pub const OPTION_FIXED_TIMESTAMP_NONE: [u8; 33] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // element 0: untouched
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // element 1: untouched
    0, // validity byte: absent (null)
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_option_fixed_timestamp_none() {
    run_list_wrapped_roundtrip_u32(
        option_fixed_timestamp_schema(),
        PyroValue::Null,
        &OPTION_FIXED_TIMESTAMP_NONE,
        &OPTION_FIXED_TIMESTAMP_INODES,
    );
}
