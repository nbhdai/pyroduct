//! Bytes are a list level of their own, so `Vec<u8>` under five more `Vec`s
//! is 6 levels, one past the limit of 5.

use pyroformat::Layout;

#[derive(Layout)]
#[pyro(Word = u32)]
struct TooDeep {
    bytes: Vec<Vec<Vec<Vec<Vec<Vec<u8>>>>>>,
}

fn main() {}
