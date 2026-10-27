//! `Map<Group<...>, Group<...>>` — groups on both sides of the map.

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
struct Label32 {
    text: String,
    weight: u32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct Label16 {
    text: String,
    weight: u32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct Annotations32 {
    entries: BTreeMap<Point32, Label32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct Annotations16 {
    entries: BTreeMap<Point16, Label16>,
}

fn assert_pyro_traits<T: Layout + Writer + Reader>() {

}

fn main() {
    assert_pyro_traits::<Point32>();
    assert_pyro_traits::<Point16>();
    assert_pyro_traits::<Label32>();
    assert_pyro_traits::<Label16>();
    assert_pyro_traits::<Annotations32>();
    assert_pyro_traits::<Annotations16>();
}
