//! Six `Vec`s around a primitive: 6 levels, one past the limit of 5.

use pyroformat::Layout;

#[derive(Layout)]
#[pyro(Word = u32)]
struct TooDeep {
    shorts: Vec<Vec<Vec<Vec<Vec<Vec<u16>>>>>>,
}

fn main() {}
