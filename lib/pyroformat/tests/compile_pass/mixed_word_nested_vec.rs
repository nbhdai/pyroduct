//! Test #[derive(Layout, Writer, Reader)] with nested structs containing Vec,
//! where the outer struct and its substruct are declared with different
//! `Word`s.
//!
//! The outer struct's `Word` governs the whole message: `Person32` writes its
//! `Vec<Address16>` (and every pointer and topology tier inside it) in `u32`
//! words, and `Person16` writes its `Vec<Address32>` in `u16` words. A
//! substruct's own `Word` only applies when it is the message itself.

use pyroformat::{Layout, Writer, Reader};

#[derive(Layout, Writer, Reader)]
#[pyro(Word = u32)]
struct Address32 {
    street: String,
    city: String,
    zip: u32,
}

#[derive(Layout, Writer, Reader)]
#[pyro(Word = u16)]
struct Address16 {
    street: String,
    city: String,
    zip: u32,
}

#[derive(Layout, Writer, Reader)]
#[pyro(Word = u32)]
struct Person32 {
    name: String,
    age: i32,
    address: Vec<Address16>,
}

#[derive(Layout, Writer, Reader)]
#[pyro(Word = u16)]
struct Person16 {
    name: String,
    age: i32,
    address: Vec<Address32>,
}

fn assert_pyro_traits<T: Layout + Writer + Reader>() {

}

fn main() {
    assert_pyro_traits::<Address32>();
    assert_pyro_traits::<Address16>();
    assert_pyro_traits::<Person32>();
    assert_pyro_traits::<Person16>();
}
