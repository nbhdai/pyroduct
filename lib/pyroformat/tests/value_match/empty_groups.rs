//! Struct-vs-value match for `src/wire/tests/empty_groups.rs` — structs with
//! no fields, in every position a struct can take.

use std::collections::BTreeMap;

use pyroformat::wire::tests::empty_groups::*;

use crate::support::read_back::assert_roundtrip;
use pyroformat::{Layout, Reader, Writer};

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
struct Empty {}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PEmptyGroup {
    a: Empty,
}

#[test]
fn value_match_empty_group() {
    let data = PEmptyGroup { a: Empty {} };
    assert_roundtrip(&data, &EMPTY_GROUP, &EMPTY_GROUP_INODES);
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PEmptyOptionGroup {
    a: Option<Empty>,
}

#[test]
fn value_match_empty_option_group_some() {
    let data = PEmptyOptionGroup { a: Some(Empty {}) };
    assert_roundtrip(
        &data,
        &EMPTY_OPTION_GROUP_SOME,
        &EMPTY_OPTION_GROUP_SOME_INODES,
    );
}

#[test]
fn value_match_empty_option_group_none() {
    let data = PEmptyOptionGroup { a: None };
    assert_roundtrip(
        &data,
        &EMPTY_OPTION_GROUP_NONE,
        &EMPTY_OPTION_GROUP_NONE_INODES,
    );
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PEmptyGroupBesideU8 {
    x: u8,
    e: Empty,
}

#[test]
fn value_match_empty_group_beside_u8() {
    let data = PEmptyGroupBesideU8 { x: 7, e: Empty {} };
    assert_roundtrip(&data, &EMPTY_GROUP_BESIDE_U8, &EMPTY_GROUP_BESIDE_U8_INODES);
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PListOfEmptyGroup {
    a: Vec<Empty>,
}

#[test]
fn value_match_list_of_empty_group() {
    let data = PListOfEmptyGroup {
        a: vec![Empty {}, Empty {}],
    };
    assert_roundtrip(&data, &LIST_OF_EMPTY_GROUP, &LIST_OF_EMPTY_GROUP_INODES);
}

#[test]
fn value_match_list_of_empty_group_no_rows() {
    let data = PListOfEmptyGroup { a: vec![] };
    assert_roundtrip(
        &data,
        &LIST_OF_EMPTY_GROUP_NO_ROWS,
        &LIST_OF_EMPTY_GROUP_NO_ROWS_INODES,
    );
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PListListOfEmptyGroup {
    a: Vec<Vec<Empty>>,
}

#[test]
fn value_match_list_list_of_empty_group() {
    let data = PListListOfEmptyGroup {
        a: vec![vec![Empty {}], vec![], vec![Empty {}, Empty {}]],
    };
    assert_roundtrip(
        &data,
        &LIST_LIST_OF_EMPTY_GROUP,
        &LIST_LIST_OF_EMPTY_GROUP_INODES,
    );
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PListOfEmptyOptionGroup {
    a: Vec<Option<Empty>>,
}

#[test]
fn value_match_list_of_empty_option_group() {
    let data = PListOfEmptyOptionGroup {
        a: vec![Some(Empty {}), None, Some(Empty {})],
    };
    assert_roundtrip(
        &data,
        &LIST_OF_EMPTY_OPTION_GROUP,
        &LIST_OF_EMPTY_OPTION_GROUP_INODES,
    );
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
struct GXAndEmpty {
    x: u8,
    e: Empty,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PEmptyGroupInListedGroup {
    a: Vec<GXAndEmpty>,
}

#[test]
fn value_match_empty_group_in_listed_group() {
    let data = PEmptyGroupInListedGroup {
        a: vec![
            GXAndEmpty { x: 1, e: Empty {} },
            GXAndEmpty { x: 2, e: Empty {} },
        ],
    };
    assert_roundtrip(
        &data,
        &EMPTY_GROUP_IN_LISTED_GROUP,
        &EMPTY_GROUP_IN_LISTED_GROUP_INODES,
    );
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PMapU8KeyEmptyGroupValue {
    a: BTreeMap<u8, Empty>,
}

#[test]
fn value_match_map_u8_key_empty_group_value() {
    let data = PMapU8KeyEmptyGroupValue {
        a: BTreeMap::from([(1u8, Empty {}), (2u8, Empty {})]),
    };
    assert_roundtrip(
        &data,
        &MAP_U8_KEY_EMPTY_GROUP_VALUE,
        &MAP_U8_KEY_EMPTY_GROUP_VALUE_INODES,
    );
}
