//! `Option<Group<Option<Group<...>>>>` — a nullable group whose own field is
//! another nullable group, so two presence bits nest.

use pyroformat::{Layout, Writer, Reader};

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct Inner32 {
    value: i64,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct Inner16 {
    value: i64,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct Middle32 {
    tag: u32,
    inner: Option<Inner32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct Middle16 {
    tag: u32,
    inner: Option<Inner16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct Outer32 {
    middle: Option<Middle32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct Outer16 {
    middle: Option<Middle16>,
}

fn assert_pyro_traits<T: Layout + Writer + Reader>() {

}

fn main() {
    assert_pyro_traits::<Inner32>();
    assert_pyro_traits::<Inner16>();
    assert_pyro_traits::<Middle32>();
    assert_pyro_traits::<Middle16>();
    assert_pyro_traits::<Outer32>();
    assert_pyro_traits::<Outer16>();
}
