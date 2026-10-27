//! Test #[derive(Layout, Writer, Reader)] with nested struct fields

use pyroformat::{Layout, Writer, Reader};

#[derive(Layout, Writer, Reader)]
#[pyro(Word = u32)]
struct Inner32 {
    value: i64,
}

#[derive(Layout, Writer, Reader)]
#[pyro(Word = u16)]
struct Inner16 {
    value: i64,
}

#[derive(Layout, Writer, Reader)]
#[pyro(Word = u32)]
struct Outer32 {
    inner: Vec<Inner32>,
    count: u32,
}

#[derive(Layout, Writer, Reader)]
#[pyro(Word = u16)]
struct Outer16 {
    inner: Vec<Inner16>,
    count: u32,
}

fn assert_pyro_traits<T: Layout + Writer + Reader>() {

}

fn main() {
    assert_pyro_traits::<Inner32>();
    assert_pyro_traits::<Inner16>();
    assert_pyro_traits::<Outer32>();
    assert_pyro_traits::<Outer16>();
}
