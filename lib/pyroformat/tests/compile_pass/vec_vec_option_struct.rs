//! `List<List<Option<Group<...>>>>` — two list levels over a nullable group.
//!
//! The deepest option-group shape the derive supports: the group's presence
//! bits sit under two levels of list topology.

use pyroformat::{Layout, Writer, Reader};

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
struct Grid32 {
    rows: Vec<Vec<Option<Point32>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct Grid16 {
    rows: Vec<Vec<Option<Point16>>>,
}

fn assert_pyro_traits<T: Layout + Writer + Reader>() {

}

fn main() {
    assert_pyro_traits::<Point32>();
    assert_pyro_traits::<Point16>();
    assert_pyro_traits::<Grid32>();
    assert_pyro_traits::<Grid16>();
}
