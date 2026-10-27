use std::collections::BTreeMap;

use pyroformat::wire::tests::map_u32::*;

use crate::support::read_back::{assert_roundtrip, assert_written};
use pyroformat::{Layout, Map, Reader, Writer};

macro_rules! value_match_map_test {
    ($test_name:ident, $struct_ty:ident, $field:expr, $buf_const:expr, $inodes_const:expr) => {
        #[test]
        fn $test_name() {
            let data = $struct_ty { a: $field };
            assert_roundtrip(&data, &$buf_const, &$inodes_const);
        }
    };
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PMapStrU32 {
    a: BTreeMap<String, u32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PMapU8U32 {
    a: BTreeMap<u8, u32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PMapStrOptionU32 {
    a: BTreeMap<String, Option<u32>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PVecMapStrU32 {
    a: Vec<BTreeMap<String, u32>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
struct GMapXY {
    x: u32,
    y: u32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PMapU8Group {
    a: BTreeMap<u8, GMapXY>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PMapGroupU8 {
    a: BTreeMap<GMapXY, u8>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PMapU8ListGroup {
    a: BTreeMap<u8, Vec<GMapXY>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PMapListGroupU8 {
    a: BTreeMap<Vec<GMapXY>, u8>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PMapU8ListBytes {
    a: BTreeMap<u8, Vec<Vec<u8>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PMapListBytesU8 {
    a: BTreeMap<Vec<Vec<u8>>, u8>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PMapListBytesListBytes {
    a: BTreeMap<Vec<Vec<u8>>, Vec<Vec<u8>>>,
}

value_match_map_test!(
    value_match_map_str_u32_single_entry,
    PMapStrU32,
    BTreeMap::from([("key".to_string(), 42u32)]),
    MAP_STR_U32_SINGLE_ENTRY,
    MAP_STR_U32_SINGLE_ENTRY_INODES
);

value_match_map_test!(
    value_match_map_str_u32_three_entries,
    PMapStrU32,
    BTreeMap::from([
        ("a".to_string(), 1u32),
        ("b".to_string(), 2u32),
        ("c".to_string(), 3u32),
    ]),
    MAP_STR_U32_THREE_ENTRIES,
    MAP_STR_U32_THREE_ENTRIES_INODES
);

value_match_map_test!(
    value_match_map_str_u32_three_entries_xyz,
    PMapStrU32,
    BTreeMap::from([
        ("x".to_string(), 10u32),
        ("y".to_string(), 20u32),
        ("z".to_string(), 30u32),
    ]),
    MAP_STR_U32_THREE_ENTRIES_XYZ,
    MAP_STR_U32_THREE_ENTRIES_XYZ_INODES
);

value_match_map_test!(
    value_match_map_str_u32_single_entry_a,
    PMapStrU32,
    BTreeMap::from([("a".to_string(), 1u32)]),
    MAP_STR_U32_SINGLE_ENTRY_A,
    MAP_STR_U32_SINGLE_ENTRY_A_INODES
);

value_match_map_test!(
    value_match_map_u8_key_u32_value,
    PMapU8U32,
    BTreeMap::from([(1u8, 100u32), (2u8, 200u32)]),
    MAP_U8_KEY_U32_VALUE,
    MAP_U8_KEY_U32_VALUE_INODES
);

value_match_map_test!(
    value_match_map_str_u32_empty,
    PMapStrU32,
    BTreeMap::new(),
    MAP_STR_U32_EMPTY,
    MAP_STR_U32_EMPTY_INODES
);

value_match_map_test!(
    value_match_map_str_option_u32_one_null,
    PMapStrOptionU32,
    BTreeMap::from([
        ("a".to_string(), Some(1u32)),
        ("b".to_string(), None),
        ("c".to_string(), Some(3u32)),
    ]),
    MAP_STR_OPTION_U32_ONE_NULL,
    MAP_STR_OPTION_U32_ONE_NULL_INODES
);

value_match_map_test!(
    value_match_list_of_map_str_u32,
    PVecMapStrU32,
    vec![BTreeMap::from([("x".to_string(), 10u32)])],
    LIST_MAP_STR_U32,
    LIST_MAP_STR_U32_INODES
);

value_match_map_test!(
    value_match_map_u8_key_group_value,
    PMapU8Group,
    BTreeMap::from([(1u8, GMapXY { x: 10, y: 20 })]),
    MAP_U8_KEY_GROUP_VALUE,
    MAP_U8_KEY_GROUP_VALUE_INODES
);

value_match_map_test!(
    value_match_map_group_key_u8_value,
    PMapGroupU8,
    BTreeMap::from([(GMapXY { x: 10, y: 20 }, 1u8)]),
    MAP_GROUP_KEY_U8_VALUE,
    MAP_GROUP_KEY_U8_VALUE_INODES
);

value_match_map_test!(
    value_match_map_u8_key_list_group_value,
    PMapU8ListGroup,
    BTreeMap::from([(1u8, vec![GMapXY { x: 10, y: 20 }])]),
    MAP_U8_KEY_LIST_GROUP_VALUE,
    MAP_U8_KEY_LIST_GROUP_VALUE_INODES
);

value_match_map_test!(
    value_match_map_list_group_key_u8_value,
    PMapListGroupU8,
    BTreeMap::from([(vec![GMapXY { x: 10, y: 20 }], 1u8)]),
    MAP_LIST_GROUP_KEY_U8_VALUE,
    MAP_LIST_GROUP_KEY_U8_VALUE_INODES
);

value_match_map_test!(
    value_match_map_u8_key_list_bytes_value,
    PMapU8ListBytes,
    BTreeMap::from([(1u8, vec![vec![10u8, 20u8]])]),
    MAP_U8_KEY_LIST_BYTES_VALUE,
    MAP_U8_KEY_LIST_BYTES_VALUE_INODES
);

value_match_map_test!(
    value_match_map_list_bytes_key_u8_value,
    PMapListBytesU8,
    BTreeMap::from([(vec![vec![10u8, 20u8]], 1u8)]),
    MAP_LIST_BYTES_KEY_U8_VALUE,
    MAP_LIST_BYTES_KEY_U8_VALUE_INODES
);

value_match_map_test!(
    value_match_map_list_bytes_key_list_bytes_value,
    PMapListBytesListBytes,
    BTreeMap::from([(vec![vec![10u8]], vec![vec![1u8], vec![2u8, 3u8]])]),
    MAP_LIST_BYTES_KEY_LIST_BYTES_VALUE,
    MAP_LIST_BYTES_KEY_LIST_BYTES_VALUE_INODES
);

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PMapStrOptionU64BesideOption {
    m: BTreeMap<String, Option<u64>>,
    z: Option<u8>,
}

#[test]
fn value_match_map_str_option_u64_beside_option_empty() {
    let data = PMapStrOptionU64BesideOption {
        m: BTreeMap::new(),
        z: None,
    };
    assert_roundtrip(
        &data,
        &MAP_STR_OPTION_U64_BESIDE_OPTION_EMPTY,
        &MAP_STR_OPTION_U64_BESIDE_OPTION_INODES,
    );
}

#[test]
fn value_match_map_str_option_u64_beside_option() {
    let data = PMapStrOptionU64BesideOption {
        m: BTreeMap::from([("a".to_string(), Some(7u64)), ("b".to_string(), None)]),
        z: Some(5),
    };
    assert_roundtrip(
        &data,
        &MAP_STR_OPTION_U64_BESIDE_OPTION,
        &MAP_STR_OPTION_U64_BESIDE_OPTION_INODES,
    );
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PMapU8MapU8U8 {
    a: BTreeMap<u8, BTreeMap<u8, u8>>,
}

value_match_map_test!(
    value_match_map_u8_key_map_value,
    PMapU8MapU8U8,
    BTreeMap::from([(1u8, BTreeMap::from([(2u8, 3u8)]))]),
    MAP_U8_KEY_MAP_VALUE,
    MAP_U8_KEY_MAP_VALUE_INODES
);

// `pyroformat::Map` writes the same bytes; the derive can't read back into
// it, so these check the write only.

#[derive(Layout, Writer, Debug)]
#[pyro(Word = u32)]
struct PPyroMapU8PyroMapU8U8 {
    a: Map<u8, Map<u8, u8>>,
}

#[test]
fn value_match_pyro_map_u8_key_pyro_map_value() {
    let data = PPyroMapU8PyroMapU8U8 {
        a: Map::from([(1u8, Map::from([(2u8, 3u8)]))]),
    };
    assert_written(&data, &MAP_U8_KEY_MAP_VALUE, &MAP_U8_KEY_MAP_VALUE_INODES);
}

#[derive(Layout, Writer, Debug)]
#[pyro(Word = u32)]
struct PMapU8PyroMapU8U8 {
    a: BTreeMap<u8, Map<u8, u8>>,
}

#[test]
fn value_match_map_u8_key_pyro_map_value() {
    let data = PMapU8PyroMapU8U8 {
        a: BTreeMap::from([(1u8, Map::from([(2u8, 3u8)]))]),
    };
    assert_written(&data, &MAP_U8_KEY_MAP_VALUE, &MAP_U8_KEY_MAP_VALUE_INODES);
}
