//! `Option<Struct>` and `Vec<Option<Struct>>` whose substruct's `Word`
//! differs from the outer struct's.
//!
//! The outer struct's `Word` governs the whole message, the option group's
//! slot and validity included: `Shape32` writes its `Point16`s in `u32`
//! words, and `Shape16` its `Point32`s in `u16` words.

use pyroformat::{Layout, Writer, Reader};

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct Point32 {
    x: i32,
    y: Option<i32>,
    name: Vec<String>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct Point16 {
    x: i32,
    y: Option<i32>,
    name: Vec<String>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct Shape32 {
    label: String,
    origin: Option<Point16>,
    vertices: Vec<Option<Point16>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct Shape16 {
    label: String,
    origin: Option<Point32>,
    vertices: Vec<Option<Point32>>,
}

fn assert_pyro_traits<T: Layout + Writer + Reader>() {

}

fn main() {
    assert_pyro_traits::<Point32>();
    assert_pyro_traits::<Point16>();
    assert_pyro_traits::<Shape32>();
    assert_pyro_traits::<Shape16>();
}
