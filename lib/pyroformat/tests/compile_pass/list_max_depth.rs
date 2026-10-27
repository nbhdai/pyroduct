//! Fields at exactly the deepest supported nesting, 5 list levels, where a
//! `String` or bytes counts as one. One more level is rejected
//! (`compile_fail/list_too_deep_*.rs`).
//!
//! The levels start over inside a struct: a group under 5 `Vec`s can hold
//! fields 5 levels deep of their own.
//!
//! `trybuild`'s `pass()` also runs this binary, so the round trips below are
//! real checks.

use pyroformat::{Layout, Reader, Realize, Writer};

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct Deepest32 {
    strings: Vec<Vec<Vec<Vec<String>>>>,
    bytes: Vec<Vec<Vec<Vec<Vec<u8>>>>>,
    #[pyro(numeric)]
    numbers: Vec<Vec<Vec<Vec<Vec<u8>>>>>,
    shorts: Vec<Vec<Vec<Vec<Vec<u16>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct Deepest16 {
    strings: Vec<Vec<Vec<Vec<String>>>>,
    bytes: Vec<Vec<Vec<Vec<Vec<u8>>>>>,
    #[pyro(numeric)]
    numbers: Vec<Vec<Vec<Vec<Vec<u8>>>>>,
    shorts: Vec<Vec<Vec<Vec<Vec<u16>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct DeepGroup {
    #[pyro(numeric)]
    a: Vec<Vec<Vec<Vec<Vec<u8>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct DeepGroups {
    g: Vec<Vec<Vec<Vec<Vec<DeepGroup>>>>>,
}

fn main() {
    let data = Deepest32 {
        strings: vec![vec![], vec![vec![vec!["a".into(), String::new()]], vec![]]],
        bytes: vec![vec![vec![vec![vec![1, 2], vec![]]]]],
        numbers: vec![vec![vec![vec![vec![3]]], vec![]]],
        shorts: vec![vec![vec![vec![vec![4, 5]]]], vec![]],
    };
    let buf = data.encode().unwrap();
    assert_eq!(Deepest32::pyro_read(&buf).unwrap().realize(), data);

    let data = Deepest16 {
        strings: vec![vec![vec![vec!["b".into()]]]],
        bytes: vec![],
        numbers: vec![vec![vec![vec![vec![6, 7]]]]],
        shorts: vec![vec![], vec![vec![vec![vec![8]]]]],
    };
    let buf = data.encode().unwrap();
    assert_eq!(Deepest16::pyro_read(&buf).unwrap().realize(), data);

    let group = |n: u8| DeepGroup { a: vec![vec![], vec![vec![vec![vec![n, n + 1]]]]] };
    let data = DeepGroups {
        g: vec![vec![], vec![vec![vec![vec![group(1), group(5)], vec![]]]], vec![vec![]]],
    };
    let buf = data.encode().unwrap();
    assert_eq!(DeepGroups::pyro_read(&buf).unwrap().realize(), data);
}
