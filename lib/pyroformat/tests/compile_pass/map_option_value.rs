//! `Map<..., Option<Primitive>>` — a map with an `Option`-wrapped primitive
//! value (`BTreeMap<String, Option<u32>>`).
//!
//! KNOWN BUG: the derive's map-value type mapping
//! (`pyroformat-derive/src/dynamic/write/type_map.rs`, `to_rust_type`) drops
//! the `Option` wrapper for a map value's `OptionPrimitive` kind, so the
//! generated writer is built for `MapListWriter<'a, String, u32, ...>`
//! instead of `MapListWriter<'a, String, Option<u32>, ...>` — the derived
//! writer type doesn't implement `PyroWriter<'a, BTreeMap<String,
//! Option<u32>>>` and this fails to compile with an unsatisfied trait bound
//! (E0277). Found via `tests/value_match/map_u32.rs`'s
//! `value_match_map_str_option_u32_one_null` test. This file documents the
//! bug and should start passing once the derive is fixed.

use pyroformat::{Layout, Writer, Reader};
use std::collections::BTreeMap;

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct MapStrOptionU32W32 {
    a: BTreeMap<String, Option<u32>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct MapStrOptionU32W16 {
    a: BTreeMap<String, Option<u32>>,
}

fn assert_pyro_traits<T: Layout + Writer + Reader>() {

}

fn main() {
    assert_pyro_traits::<MapStrOptionU32W32>();
    assert_pyro_traits::<MapStrOptionU32W16>();
}
