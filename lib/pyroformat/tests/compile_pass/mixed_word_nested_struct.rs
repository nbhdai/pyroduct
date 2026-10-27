//! A bare (non-`Vec`) substruct field whose `Word` differs from the outer
//! struct's.
//!
//! The outer struct's `Word` governs the whole message: `Line32` lays out and
//! writes its `Point16` fields' slots in `u32` words, and `Line16` its
//! `Point32` fields' slots in `u16` words.

use pyroformat::{Layout, Writer, Reader};

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct Point32 {
    label: String,
    x: i32,
    y: i32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct Point16 {
    label: String,
    x: i32,
    y: i32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct Line32 {
    start: Point16,
    end: Point16,
    tags: Vec<String>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct Line16 {
    start: Point32,
    end: Point32,
    tags: Vec<String>,
}

fn assert_pyro_traits<T: Layout + Writer + Reader>() {

}

fn main() {
    assert_pyro_traits::<Point32>();
    assert_pyro_traits::<Point16>();
    assert_pyro_traits::<Line32>();
    assert_pyro_traits::<Line16>();
}
