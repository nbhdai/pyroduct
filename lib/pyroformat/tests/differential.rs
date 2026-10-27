//! Shapes where the derive path and the value path disagree, found by
//! differential fuzzing. Each test asserts the two agree; until a bug is
//! fixed, its failure message says which side broke (see
//! `support/differential.rs`).
//!
//! Each doc comment records what was observed and which side the evidence
//! points at. That is a lead, not a verdict: the value path defines the
//! format, but here it is sometimes the side that panics.
//!
//! Cases that blow up memory live in `memory_bugs.rs` and run isolated;
//! malformed-buffer cases live in `corrupt_buffers.rs`.

#[path = "support/differential.rs"]
mod differential;

use std::collections::BTreeMap;

use differential::{assert_agree, assert_both_reject};
use half::f16;
use pyroformat::value::Time;
use pyroformat::{BitVec, Layout, PyroValue, Reader, Writer};

// ===========================================================================
// Absent bare `Option<Group>`: more shapes of the bug pinned in
// `src/wire/tests/primitive_group_option_*.rs` (`option_group_absent_*`).
// The derive's `Option<G>` writer skips the absent group's fields, so their
// lanes never get start pointers (left 0) or alignment padding. Both sides
// read both byte streams; only the bytes differ. Side: derive writer.
// ===========================================================================

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct BoolLane {
    #[pyro(allow(vec_bool))]
    v: Vec<bool>,
    x: u8,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct AbsentBoolLane {
    a: Option<BoolLane>,
    b: Vec<u32>,
}

/// The absent group's lane is a bit-packed `Vec<bool>`.
#[test]
fn absent_option_group_bool_lane() {
    assert_agree(&AbsentBoolLane {
        a: None,
        b: vec![1],
    });
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct MapLane {
    m: BTreeMap<String, u32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct AbsentMapLane {
    a: Option<MapLane>,
    b: String,
}

/// The absent group holds a map: both its key and value lanes are skipped.
#[test]
fn absent_option_group_map_lanes() {
    assert_agree(&AbsentMapLane {
        a: None,
        b: "q".into(),
    });
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct FixedMap {
    m: BTreeMap<u8, u8>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct AbsentFixedMap {
    a: Option<FixedMap>,
    t: u8,
}

/// The absent group's only field is a map of fixed-width entries: not just
/// variable-length leaves are affected.
#[test]
fn absent_option_group_fixed_width_map() {
    assert_agree(&AbsentFixedMap { a: None, t: 1 });
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct Named {
    s: String,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct Rows {
    g: Vec<Named>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct AbsentRows {
    a: Option<Rows>,
    b: u8,
}

/// The absent group holds a list of groups.
#[test]
fn absent_option_group_list_of_groups() {
    assert_agree(&AbsentRows { a: None, b: 1 });
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct TimeLanes {
    t: Option<Time>,
    h: f16,
    v: Vec<f16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct AbsentTimeLanes {
    a: Option<TimeLanes>,
    #[pyro(numeric)]
    b: Vec<u8>,
}

/// The absent group mixes a 16-byte `Time` slot, an `f16` and an `f16` list.
#[test]
fn absent_option_group_time_and_f16_lanes() {
    assert_agree(&AbsentTimeLanes {
        a: None,
        b: vec![1, 2, 3],
    });
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct Bits {
    b: BitVec,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct AbsentBits {
    a: Option<Bits>,
    c: String,
}

/// The absent group holds a `BitVec`.
#[test]
fn absent_option_group_bitvec() {
    assert_agree(&AbsentBits {
        a: None,
        c: "zz".into(),
    });
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct Shorts {
    v: Vec<u16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct Siblings {
    a: Option<Shorts>,
    b: Option<Shorts>,
    c: Option<Shorts>,
}

/// Present, absent, present siblings: only the middle one's pointer is lost,
/// and the value path puts it where the third one starts.
#[test]
fn absent_option_group_between_present_siblings() {
    assert_agree(&Siblings {
        a: Some(Shorts { v: vec![7] }),
        b: None,
        c: Some(Shorts { v: vec![8, 9] }),
    });
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct Inner16 {
    v: Vec<u16>,
    s: String,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct Outer32 {
    a: Option<Inner16>,
    b: Vec<Option<Inner16>>,
    t: String,
}

/// Mixed words: a `u16` group absent inside a `u32` message.
#[test]
fn absent_option_group_mixed_word() {
    assert_agree(&Outer32 {
        a: None,
        b: vec![],
        t: "t".into(),
    });
}

// ===========================================================================
// `Option<bool>`
// ===========================================================================

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct OptBools {
    a: Vec<Option<bool>>,
}

/// One-element `Vec<Option<bool>>` holding a `Some`: the derive writer
/// panics, index out of bounds writing the trailing partial validity byte
/// (`src/wire/writers/validity.rs`, `write_iter`). `[None]`, two elements
/// and longer all work. Side: derive writer.
#[test]
fn vec_option_bool_single_some() {
    assert_agree(&OptBools {
        a: vec![Some(true)],
    });
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct NestedOptBools {
    a: Vec<Vec<Option<bool>>>,
}

/// The same panic one level down: one `Some` in total.
#[test]
fn vec_vec_option_bool_single_some() {
    assert_agree(&NestedOptBools {
        a: vec![vec![], vec![Some(true)]],
    });
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct OptBoolMap {
    m: BTreeMap<u8, Option<bool>>,
}

/// The same panic through a map's value column (`validity.rs:70`).
#[test]
fn map_option_bool_single_some() {
    assert_agree(&OptBoolMap {
        m: [(1, Some(true))].into(),
    });
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct OptBool {
    g: Option<bool>,
}

/// A bare `Option<bool>` holding `Some(false)`: the derive writes `[0, 1]`
/// (value 0, present), the value path `[0, 0]` and reads the derive's bytes
/// as `Null`. `Some(true)` and `None` agree. The derive reads its own bytes
/// but not the value path's. Side: the two disagree on whether a present
/// `false` sets the validity bit — most likely the value path drops it.
#[test]
fn option_bool_some_false() {
    assert_agree(&OptBool { g: Some(false) });
}

// ===========================================================================
// Fixed arrays
// ===========================================================================

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct Times {
    a: [Time; 2],
}

/// `[Time; N]` with `N > 1`: the value path reads and writes it as one
/// `Timestamp` (16 bytes); the derive writes `N * 16` bytes and round-trips
/// them. `[Time; 1]` and every other element type agree. Side: value path
/// (or the `Timestamp` array inode dropping its length).
#[test]
fn time_array() {
    assert_agree(&Times {
        a: [Time(1), Time(2)],
    });
}

// ===========================================================================
// Maps below a list
// ===========================================================================

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct MapOfMaps {
    a: BTreeMap<u8, BTreeMap<u8, u8>>,
}

/// A map as a map's value: the inner map is iterated, so its slot points at
/// a tier of per-map entry counts. Was: the derive's `BTreeMap` writer left
/// that slot zeroed and wrote no tier. Pinned in
/// `src/wire/tests/map_u32.rs` (`MAP_U8_KEY_MAP_VALUE`).
#[test]
fn map_of_maps() {
    assert_agree(&MapOfMaps {
        a: [(1, [(2, 3)].into())].into(),
    });
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct HasMap {
    m: BTreeMap<u8, u8>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct ListOfHasMap {
    l: Vec<HasMap>,
}

/// A struct with a map, in a list: the same tier as [`map_of_maps`], for a
/// map one list level down. Pinned in `src/wire/tests/map_nested_u32.rs`
/// (`LIST_GROUP_MAP_U8_U8`).
#[test]
fn list_of_struct_with_map() {
    assert_agree(&ListOfHasMap {
        l: vec![HasMap { m: [(1, 2)].into() }],
    });
}

/// With no rows the tier is empty, but its pointer is still laid out. Pinned
/// in `src/wire/tests/map_nested_u32.rs` (`LIST_GROUP_MAP_U8_U8_EMPTY`).
#[test]
fn list_of_struct_with_map_empty() {
    assert_agree(&ListOfHasMap { l: vec![] });
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct ListListMap {
    a: Vec<Vec<BTreeMap<u8, u8>>>,
}

/// `[[]]`: the entry-count level has no nodes and isn't written. Was: the
/// value reader took the last level written as the leaf level and read
/// `[{}]`; `eat_compound_values` now splits the leaf off by the schema's
/// level count. Pinned in `src/wire/tests/map_nested_u32.rs`
/// (`LIST_LIST_MAP_U8_U8_EMPTY_INNER`).
#[test]
fn list_list_map_empty_inner() {
    assert_agree(&ListListMap { a: vec![vec![]] });
}

#[derive(Layout, Writer, Reader, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[pyro(Word = u32)]
struct Key {
    a: u8,
    s: String,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct ListOfStructKeyMaps {
    a: Vec<BTreeMap<Key, u8>>,
}

/// Struct keys one list level down. Was: `from_schema` dropped the key's
/// field names under a `List`, and the key group had no count of its own.
/// Pinned in `src/wire/tests/map_nested_u32.rs`
/// (`LIST_MAP_GROUP_KEY_U8_VALUE`).
#[test]
fn list_of_struct_key_maps() {
    assert_agree(&ListOfStructKeyMaps {
        a: vec![
            [(
                Key {
                    a: 1,
                    s: "a".into(),
                },
                1,
            )]
            .into(),
        ],
    });
}

// ===========================================================================
// `u16` words past 64 KiB. A message too big for the word must be an
// `OffsetOverflow` from both writers; where the data does fit, the two must
// still agree. Each rejected shape has a `u32` twin that carries the same
// data to the value writer.
// ===========================================================================

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct TwoStrings {
    a: String,
    b: String,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct TwoStrings32 {
    a: String,
    b: String,
}

/// The empty trailing lane `b` would start at offset 65536: nothing is
/// written there, but its start pointer can't hold it.
#[test]
fn u16_empty_lane_starting_at_65536() {
    assert_both_reject(
        &TwoStrings {
            a: "x".repeat(65528),
            b: String::new(),
        },
        &TwoStrings32 {
            a: "x".repeat(65528),
            b: String::new(),
        },
    );
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct Lists16 {
    #[pyro(numeric)]
    v: Vec<Vec<u8>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct Lists32 {
    #[pyro(numeric)]
    v: Vec<Vec<u8>>,
}

/// An inner list of exactly 65535 bytes: its topology would carry `0xFFFF`,
/// the tier reset, so the message is past the limit.
#[test]
fn u16_inner_list_of_65535_bytes() {
    assert_both_reject(
        &Lists16 {
            v: vec![vec![1; 65535]],
        },
        &Lists32 {
            v: vec![vec![1; 65535]],
        },
    );
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct StrThenWide {
    a: String,
    b: Vec<u64>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct StrThenWide32 {
    a: String,
    b: Vec<u64>,
}

/// About 65.6 KB in all: the lane's length word would wrap.
#[test]
fn u16_offsets_wrap_past_64k() {
    assert_both_reject(
        &StrThenWide {
            a: "x".repeat(10),
            b: vec![0x0102030405060708; 8192],
        },
        &StrThenWide32 {
            a: "x".repeat(10),
            b: vec![0x0102030405060708; 8192],
        },
    );
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct ManyEmpty16 {
    s: Vec<Vec<u8>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct ManyEmpty32 {
    s: Vec<Vec<u8>>,
}

/// 32766 empty inner lists: about 64 KiB of topology, just past the limit.
#[test]
fn u16_many_empty_inner_lists() {
    assert_both_reject(
        &ManyEmpty16 {
            s: vec![vec![]; 32766],
        },
        &ManyEmpty32 {
            s: vec![vec![]; 32766],
        },
    );
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct ListOfMaps16 {
    a: Vec<BTreeMap<u8, u8>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct ListOfMaps32 {
    a: Vec<BTreeMap<u8, u8>>,
}

/// 300 maps of 256 entries, about 154 KB. Before the overflow check the
/// derive panicked one byte past its buffer here, which only showed because
/// the message is this far past the limit.
#[test]
fn u16_list_of_maps_past_64k() {
    let map: BTreeMap<u8, u8> = (0..=255).map(|k| (k, k)).collect();
    assert_both_reject(
        &ListOfMaps16 {
            a: vec![map.clone(); 300],
        },
        &ListOfMaps32 { a: vec![map; 300] },
    );
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct OneString16 {
    s: String,
}

/// A string that cannot fit a `u16` offset must be an error, from both
/// writers: the derive panics, and the value writer emits wrapped bytes that
/// read back truncated.
#[test]
fn u16_oversize_string_is_an_error() {
    fn outcome(r: Result<Result<usize, String>, String>) -> String {
        match r {
            Ok(Ok(len)) => format!("returned Ok ({len} bytes)"),
            Ok(Err(e)) => format!("returned Err: {e}"),
            Err(p) => format!("PANICKED: {p}"),
        }
    }
    let data = OneString16 {
        s: "x".repeat(65536),
    };
    let derive = differential::guard(|| {
        data.encode()
            .map(|b| b.payload().len())
            .map_err(|e| format!("{e:?}"))
    });
    // A one-field struct's value is the field's own value.
    let value = PyroValue::Str(data.s.as_str().into());
    let written =
        differential::guard(|| differential::value_write::<OneString16>(&value).map(|b| b.len()));
    assert!(
        matches!(derive, Ok(Err(_))) && matches!(written, Ok(Err(_))),
        "both writers must return Err\n  derive writer: {}\n  value writer: {}",
        outcome(derive),
        outcome(written),
    );
}

// ===========================================================================
// Groups repeated by an ancestor own their instance count. A nesting-0 group
// under a list, a listed group or a map used to read its count from offset 0,
// which only held the right number by coincidence. Side: layout.
// ===========================================================================

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
struct CountInner {
    a: u8,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
struct CountOuter {
    inner: CountInner,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct ListedGroupAfterList {
    x: Vec<u8>,
    l: Vec<CountOuter>,
}

/// Offset 0 was `x`'s pointer.
#[test]
fn nested_group_after_another_field() {
    assert_agree(&ListedGroupAfterList {
        x: vec![7, 8, 9],
        l: vec![CountOuter {
            inner: CountInner { a: 1 },
        }],
    });
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct DoubleListedGroup {
    l: Vec<Vec<CountOuter>>,
}

/// Offset 0 was the outer list's topology pointer.
#[test]
fn nested_group_two_lists_down() {
    assert_agree(&DoubleListedGroup {
        l: vec![vec![CountOuter {
            inner: CountInner { a: 1 },
        }]],
    });
}

#[derive(Layout, Writer, Reader, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct CountKey {
    a: u8,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
struct HasKeyMap {
    m: BTreeMap<CountKey, u8>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct ListOfKeyMaps {
    l: Vec<HasKeyMap>,
}

/// A struct-keyed map in a listed group: the key shared the map's slot,
/// which is a topology pointer there.
#[test]
fn struct_key_map_in_listed_group() {
    assert_agree(&ListOfKeyMaps {
        l: vec![HasKeyMap {
            m: [(CountKey { a: 1 }, 2)].into(),
        }],
    });
}
