//! Malformed messages: a valid message with one byte changed. Neither reader
//! may panic on bytes it is handed, and the two must agree on whether the
//! message is readable. Found by single-byte mutation fuzzing; each test
//! pins one mutation.

#[path = "support/differential.rs"]
mod differential;

use std::collections::BTreeMap;
use std::fmt::Debug;

use differential::{Read, derive_read, value_read};
use pyroformat::value::Time;
use pyroformat::{Layout, Reader, Writer};

/// `data`'s message with byte `at` set to `to`, which must change it.
fn mutated<T: Writer>(data: &T, at: usize, to: u8) -> Vec<u8> {
    let mut bytes = data.encode().unwrap().payload().to_vec();
    assert_ne!(bytes[at], to, "the mutation must change the message");
    bytes[at] = to;
    bytes
}

/// Neither reader panics on `bytes`, and both accept or both reject them.
#[track_caller]
fn assert_readers_agree<T: Reader + Layout + Debug>(bytes: &[u8]) {
    let derive = derive_read::<T>(bytes);
    let value = value_read::<T>(bytes);
    let panicked = matches!(derive, Read::Panicked(_)) || matches!(value, Read::Panicked(_));
    let agree = matches!(derive, Read::Accepted(_)) == matches!(value, Read::Accepted(_));
    assert!(
        !panicked && agree,
        "readers disagree on {bytes:?}\n  derive reader: {derive:?}\n  value reader: {value:?}"
    );
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct Lists3 {
    s: Vec<Vec<Vec<u8>>>,
}

/// The topology's word count raised from 1 to 2: the value reader panics
/// (`panic!("Throw a nice error here")`, `src/wire/nesting/mod.rs`); the
/// derive reader accepts and reads an empty list. Over every single-byte
/// mutation of this message, 3570 of 6912 panic the value reader.
#[test]
fn corrupt_topology_count() {
    let bytes = mutated(
        &Lists3 {
            s: vec![vec![vec![1]]],
        },
        2,
        2,
    );
    assert_readers_agree::<Lists3>(&bytes);
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct Text {
    s: String,
}

/// A string's first byte made `0x80`, invalid UTF-8: the value reader
/// rejects it (`InvalidUtf8`); the derive reader accepts and reads `""`,
/// since it checks UTF-8 lazily and maps a bad string to empty
/// (`src/refs/str_leaf.rs`). A silent data change rather than an error.
#[test]
fn invalid_utf8() {
    let bytes = mutated(&Text { s: "hi".into() }, 4, 0x80);
    assert_readers_agree::<Text>(&bytes);
}

#[derive(Layout, Writer, Reader, Debug, PartialEq, Clone)]
#[pyro(Word = u16)]
struct Point {
    x: u8,
    n: String,
    l: Vec<u16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq, Clone)]
#[pyro(Word = u16)]
struct Scene {
    pts: Vec<Point>,
    m: BTreeMap<String, Vec<Point>>,
    o: Vec<Option<Point>>,
    t: Vec<Time>,
    ot: Option<Time>,
}

fn point(i: usize) -> Point {
    Point {
        x: i as u8,
        n: "n".repeat(i % 5),
        l: vec![i as u16; i % 4],
    }
}

fn scene(k: usize) -> Scene {
    Scene {
        pts: (0..k).map(point).collect(),
        m: (0..k)
            .map(|i| (format!("k{i}"), (0..i % 4).map(point).collect()))
            .collect(),
        o: (0..k)
            .map(|i| if i % 2 == 0 { Some(point(i)) } else { None })
            .collect(),
        t: (0..k).map(|i| Time(i as i128)).collect(),
        ot: if k % 2 == 1 { Some(Time(5)) } else { None },
    }
}

/// A row count raised: the derive reader accepts the message, then
/// `realize()` panics unwrapping a row that isn't there
/// (`RowsRef::iter`, `src/interface.rs`); the value reader accepts it.
/// 3075 of 51968 single-byte mutations of this message do this.
#[test]
fn corrupt_row_count_panics_derive_realize() {
    let bytes = mutated(&scene(1), 4, 2);
    assert_readers_agree::<Scene>(&bytes);
}

/// A count zeroed: the value reader panics on a count it never checked —
/// `attempt to subtract with overflow` in a debug build, `capacity overflow`
/// reserving from the wrapped result in release. The derive reader reads it.
#[test]
fn corrupt_count_overflows_value_reader_capacity() {
    let bytes = mutated(&scene(5), 156, 0);
    assert_readers_agree::<Scene>(&bytes);
}

/// A pointer changed so the topology is deeper than the field allows: the
/// derive reader rejects it (`InvalidTopologyDepth`), the value reader
/// accepts it. 34 of 51968 mutations; the reverse (derive accepts, value
/// rejects) shows up in 1024.
#[test]
fn corrupt_topology_depth() {
    let bytes = mutated(&scene(1), 34, 4);
    assert_readers_agree::<Scene>(&bytes);
}
