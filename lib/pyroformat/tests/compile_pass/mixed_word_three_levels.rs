//! Three levels of nesting with the `Word` alternating at every level.
//!
//! Only the outermost struct's `Word` counts: `Org32` writes everything,
//! its `Team16`s and their `Member32`s included, in `u32` words, and `Org16`
//! writes its `Team32`s and their `Member16`s in `u16` words. The middle
//! struct's `Word` does not take over for the level below it.

use pyroformat::{Layout, Writer, Reader};

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct Member32 {
    name: String,
    roles: Vec<String>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct Member16 {
    name: String,
    roles: Vec<String>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct Team32 {
    title: String,
    members: Vec<Member16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct Team16 {
    title: String,
    members: Vec<Member32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct Org32 {
    id: u64,
    teams: Vec<Team16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct Org16 {
    id: u64,
    teams: Vec<Team32>,
}

fn assert_pyro_traits<T: Layout + Writer + Reader>() {

}

fn main() {
    assert_pyro_traits::<Member32>();
    assert_pyro_traits::<Member16>();
    assert_pyro_traits::<Team32>();
    assert_pyro_traits::<Team16>();
    assert_pyro_traits::<Org32>();
    assert_pyro_traits::<Org16>();
}
