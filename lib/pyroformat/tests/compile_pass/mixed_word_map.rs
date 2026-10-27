//! A struct-valued map whose value struct's `Word` differs from the outer
//! struct's.
//!
//! The outer struct's `Word` governs the whole message, the map's count and
//! the value group's slots included: `Scene32` writes its `Point16` values in
//! `u32` words, and `Scene16` its `Point32` values in `u16` words.

use pyroformat::{Layout, Writer, Reader};
use std::collections::BTreeMap;

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct Point32 {
    x: i32,
    y: i32,
    notes: Vec<String>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct Point16 {
    x: i32,
    y: i32,
    notes: Vec<String>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct Scene32 {
    name: String,
    markers: BTreeMap<String, Point16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct Scene16 {
    name: String,
    markers: BTreeMap<String, Point32>,
}

fn assert_pyro_traits<T: Layout + Writer + Reader>() {

}

fn main() {
    assert_pyro_traits::<Point32>();
    assert_pyro_traits::<Point16>();
    assert_pyro_traits::<Scene32>();
    assert_pyro_traits::<Scene16>();
}
