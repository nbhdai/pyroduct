//! `Map<..., Vec<Group<...>>>` — a map whose value is a `Vec` of a
//! struct-shaped (`Group`) type (`BTreeMap<u8, Vec<Point>>`).
//!
//! KNOWN BUG: the derive's map-value type mapping
//! (`pyroformat-derive/src/dynamic/write/type_map.rs`, `to_rust_type`)
//! ignores `vec_depth` for struct-shaped (`MissingStruct`/`ContainedStruct`)
//! map value types, so the generated writer is built for
//! `MapListWriter<'a, u8, Point, ...>` instead of `MapListWriter<'a, u8,
//! Vec<Point>, ...>` — this fails to compile with an unsatisfied trait
//! bound (E0277) plus a cascade of type mismatches. Found via
//! `tests/value_match/map_u32.rs`'s `value_match_map_u8_key_list_group_value`
//! test. This file documents the bug and should start passing once the
//! derive is fixed.

use pyroformat::{Layout, Writer, Reader};
use std::collections::BTreeMap;

#[derive(Layout, Writer, Reader, Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
#[pyro(Word = u32)]
struct Point32 {
    x: i32,
    y: i32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
#[pyro(Word = u16)]
struct Point16 {
    x: i32,
    y: i32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct MapU8ToVecPoint32 {
    a: BTreeMap<u8, Vec<Point32>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct MapU8ToVecPoint16 {
    a: BTreeMap<u8, Vec<Point16>>,
}

fn assert_pyro_traits<T: Layout + Writer + Reader>() {

}

fn main() {
    assert_pyro_traits::<Point32>();
    assert_pyro_traits::<Point16>();
    assert_pyro_traits::<MapU8ToVecPoint32>();
    assert_pyro_traits::<MapU8ToVecPoint16>();
}
