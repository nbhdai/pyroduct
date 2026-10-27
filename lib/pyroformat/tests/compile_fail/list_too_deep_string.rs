//! A `String` is a list level of its own, so five `Vec`s around one are 6
//! levels, one past the limit of 5.

use pyroformat::Layout;

#[derive(Layout)]
#[pyro(Word = u32)]
struct TooDeep {
    strings: Vec<Vec<Vec<Vec<Vec<String>>>>>,
}

fn main() {}
