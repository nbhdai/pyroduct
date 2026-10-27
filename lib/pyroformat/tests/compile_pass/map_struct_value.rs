//! `Map<..., Group<...>>` — a struct-valued map (`BTreeMap<String, Point>`).
//!
//! The map's own inode is followed by its key and value inodes; a group-shaped
//! value contributes the group inode plus one child inode per leaf field.

use pyroformat::{Layout, Writer, Reader};
use std::collections::BTreeMap;

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct Point32 {
    x: i32,
    y: i32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct Point16 {
    x: i32,
    y: i32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct Scene32 {
    name: String,
    markers: BTreeMap<String, Point32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct Scene16 {
    name: String,
    markers: BTreeMap<String, Point16>,
}

fn assert_pyro_traits<T: Layout + Writer + Reader>() {

}

fn main() {
    assert_pyro_traits::<Point32>();
    assert_pyro_traits::<Point16>();
    assert_pyro_traits::<Scene32>();
    assert_pyro_traits::<Scene16>();
}
