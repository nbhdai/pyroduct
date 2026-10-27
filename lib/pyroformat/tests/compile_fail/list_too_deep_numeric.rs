//! Six `Vec`s around a `#[pyro(numeric)]` `u8`: 6 levels, one past the limit
//! of 5.

use pyroformat::Layout;

#[derive(Layout)]
#[pyro(Word = u32)]
struct TooDeep {
    #[pyro(numeric)]
    numbers: Vec<Vec<Vec<Vec<Vec<Vec<u8>>>>>>,
}

fn main() {}
