//! `Option<Group<List<Group<...>>>>` — a nullable group holding a list of
//! required groups, so a group list sits beneath a presence bit.

use pyroformat::{Layout, Writer, Reader};

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct Item32 {
    sku: String,
    qty: u32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct Item16 {
    sku: String,
    qty: u32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct Order32 {
    id: u64,
    items: Vec<Item32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct Order16 {
    id: u64,
    items: Vec<Item16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct Session32 {
    order: Option<Order32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct Session16 {
    order: Option<Order16>,
}

fn assert_pyro_traits<T: Layout + Writer + Reader>() {

}

fn main() {
    assert_pyro_traits::<Item32>();
    assert_pyro_traits::<Item16>();
    assert_pyro_traits::<Order32>();
    assert_pyro_traits::<Order16>();
    assert_pyro_traits::<Session32>();
    assert_pyro_traits::<Session16>();
}
