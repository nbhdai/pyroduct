//! Struct-vs-value match for `src/wire/tests/map_nested_u32.rs` —
//! `List<Map<u8, u8>>`, `List<List<Map<u8, u8>>>`, and `List<List<List<Map<u8, u8>>>>`.

use std::collections::BTreeMap;

use pyroformat::wire::tests::map_nested_u32::*;

use crate::support::read_back::{assert_roundtrip, assert_written};
use pyroformat::{Layout, Map, Reader, Writer};

macro_rules! value_match_map_nested_test {
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
struct PListMapU8U8 {
    a: Vec<BTreeMap<u8, u8>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PListListMapU8U8 {
    a: Vec<Vec<BTreeMap<u8, u8>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PListListListMapU8U8 {
    a: Vec<Vec<Vec<BTreeMap<u8, u8>>>>,
}

value_match_map_nested_test!(
    value_match_list_of_map_u8_u8,
    PListMapU8U8,
    vec![
        BTreeMap::from([(1u8, 10u8), (2u8, 20u8)]),
        BTreeMap::from([(3u8, 30u8)]),
    ],
    LIST_MAP_U8_U8,
    LIST_MAP_U8_U8_INODES
);

value_match_map_nested_test!(
    value_match_list_of_list_of_map_u8_u8,
    PListListMapU8U8,
    vec![vec![
        BTreeMap::from([(1u8, 10u8)]),
        BTreeMap::from([(2u8, 20u8)]),
    ]],
    LIST_LIST_MAP_U8_U8,
    LIST_LIST_MAP_U8_U8_INODES
);

value_match_map_nested_test!(
    value_match_list_of_list_of_list_of_map_u8_u8,
    PListListListMapU8U8,
    vec![
        vec![
            vec![BTreeMap::from([(1u8, 10u8), (2u8, 20u8)]), BTreeMap::new(),],
            vec![BTreeMap::from([(3u8, 30u8)])],
        ],
        vec![vec![BTreeMap::from([(4u8, 40u8), (5u8, 50u8)])]],
    ],
    LIST_LIST_LIST_MAP_U8_U8,
    LIST_LIST_LIST_MAP_U8_U8_INODES
);

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
struct GHasMapU8U8 {
    m: BTreeMap<u8, u8>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PListGroupMapU8U8 {
    a: Vec<GHasMapU8U8>,
}

value_match_map_nested_test!(
    value_match_list_of_group_map_u8_u8,
    PListGroupMapU8U8,
    vec![GHasMapU8U8 {
        m: BTreeMap::from([(1u8, 2u8)]),
    }],
    LIST_GROUP_MAP_U8_U8,
    LIST_GROUP_MAP_U8_U8_INODES
);

value_match_map_nested_test!(
    value_match_list_of_group_map_u8_u8_empty,
    PListGroupMapU8U8,
    vec![],
    LIST_GROUP_MAP_U8_U8_EMPTY,
    LIST_GROUP_MAP_U8_U8_EMPTY_INODES
);

// `pyroformat::Map` writes the same bytes; the derive can't read back into
// it, so these check the write only.

#[derive(Layout, Writer, Debug)]
#[pyro(Word = u32)]
struct PListPyroMapU8U8 {
    a: Vec<Map<u8, u8>>,
}

#[test]
fn value_match_list_of_pyro_map_u8_u8() {
    let data = PListPyroMapU8U8 {
        a: vec![
            Map::from([(1u8, 10u8), (2u8, 20u8)]),
            Map::from([(3u8, 30u8)]),
        ],
    };
    assert_written(&data, &LIST_MAP_U8_U8, &LIST_MAP_U8_U8_INODES);
}

#[derive(Layout, Writer, Debug)]
struct GHasPyroMapU8U8 {
    m: Map<u8, u8>,
}

#[derive(Layout, Writer, Debug)]
#[pyro(Word = u32)]
struct PListGroupPyroMapU8U8 {
    a: Vec<GHasPyroMapU8U8>,
}

#[test]
fn value_match_list_of_group_pyro_map_u8_u8() {
    let data = PListGroupPyroMapU8U8 {
        a: vec![GHasPyroMapU8U8 {
            m: Map::from([(1u8, 2u8)]),
        }],
    };
    assert_written(&data, &LIST_GROUP_MAP_U8_U8, &LIST_GROUP_MAP_U8_U8_INODES);
}

#[test]
fn value_match_list_of_group_pyro_map_u8_u8_empty() {
    let data = PListGroupPyroMapU8U8 { a: vec![] };
    assert_written(
        &data,
        &LIST_GROUP_MAP_U8_U8_EMPTY,
        &LIST_GROUP_MAP_U8_U8_EMPTY_INODES,
    );
}

#[derive(Layout, Writer, Reader, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct GKeyAS {
    a: u8,
    s: String,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PListMapGroupKeyU8 {
    a: Vec<BTreeMap<GKeyAS, u8>>,
}

value_match_map_nested_test!(
    value_match_list_of_map_group_key_u8_value,
    PListMapGroupKeyU8,
    vec![BTreeMap::from([(
        GKeyAS {
            a: 1,
            s: "a".into(),
        },
        1u8,
    )])],
    LIST_MAP_GROUP_KEY_U8_VALUE,
    LIST_MAP_GROUP_KEY_U8_VALUE_INODES
);

value_match_map_nested_test!(
    value_match_list_of_list_of_map_u8_u8_empty_inner,
    PListListMapU8U8,
    vec![vec![]],
    LIST_LIST_MAP_U8_U8_EMPTY_INNER,
    LIST_LIST_MAP_U8_U8_EMPTY_INNER_INODES
);
