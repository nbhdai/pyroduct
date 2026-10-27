//! Test #[derive(Layout, Writer, Reader)] with basic primitive types

use pyroformat::{Layout, Reader, Writer};

#[derive(Layout, Writer, Reader)]
#[pyro(Word = u32)]
struct SimpleStruct32 {
    id: u32,
    count: i32,
    active: bool,
    score: f64,
}

#[derive(Layout, Writer, Reader)]
#[pyro(Word = u16)]
struct SimpleStruct16 {
    id: u32,
    count: i32,
    active: bool,
    score: f64,
}

fn assert_pyro_traits<T: Layout + Writer + Reader>() {}

fn main() {
    assert_pyro_traits::<SimpleStruct16>();
    assert_pyro_traits::<SimpleStruct32>();
}
