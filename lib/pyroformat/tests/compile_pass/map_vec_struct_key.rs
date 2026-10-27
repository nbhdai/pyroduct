//! `Map<Vec<Group<...>>, ...>` — a map whose key is a `Vec` of a
//! struct-shaped (`Group`) type (`BTreeMap<Vec<Point>, u8>`).
//!
//! KNOWN BUG: the mirror of `map_vec_struct_value`'s bug — the derive's
//! map-key type mapping (`pyroformat-derive/src/dynamic/write/type_map.rs`,
//! `to_rust_type`) ignores `vec_depth` for struct-shaped map key types, so
//! the generated writer is built for `MapListWriter<'a, Point, u8, ...>`
//! instead of `MapListWriter<'a, Vec<Point>, u8, ...>` — this fails to
//! compile with an unsatisfied trait bound (E0277) plus a cascade of type
//! mismatches. Found via `tests/value_match/map_u32.rs`'s
//! `value_match_map_list_group_key_u8_value` test. This file documents the
//! bug and should start passing once the derive is fixed.

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
struct MapVecPointToU8W32 {
    a: BTreeMap<Vec<Point32>, u8>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct MapVecPointToU8W16 {
    a: BTreeMap<Vec<Point16>, u8>,
}

fn assert_pyro_traits<T: Layout + Writer + Reader>() {

}

fn main() {
    assert_pyro_traits::<Point32>();
    assert_pyro_traits::<Point16>();
    assert_pyro_traits::<MapVecPointToU8W32>();
    assert_pyro_traits::<MapVecPointToU8W16>();
}
