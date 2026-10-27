//! `Map<Group<...>, ...>` — a struct-*keyed* map (`BTreeMap<Point, u32>`).
//!
//! The mirror of `map_struct_value`: the group sits on the key side, so the
//! group inode and its children follow the map inode before the value inode.

use pyroformat::{Layout, Writer, Reader};
use std::collections::BTreeMap;

#[derive(Layout, Writer, Reader, Debug, PartialEq, PartialOrd, Eq, Ord)]
#[pyro(Word = u32)]
struct Point32 {
    x: i32,
    y: i32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq, PartialOrd, Eq, Ord)]
#[pyro(Word = u16)]
struct Point16 {
    x: i32,
    y: i32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct Heatmap32 {
    name: String,
    weights: BTreeMap<Point32, u32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct Heatmap16 {
    name: String,
    weights: BTreeMap<Point16, u32>,
}

fn assert_pyro_traits<T: Layout + Writer + Reader>() {

}

fn main() {
    assert_pyro_traits::<Point32>();
    assert_pyro_traits::<Point16>();
    assert_pyro_traits::<Heatmap32>();
    assert_pyro_traits::<Heatmap16>();
}
