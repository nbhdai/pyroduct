//! `Map<..., Vec<Vec<u8>>>` — a map value that's a `List` of `Bytes`
//! (inner `Vec<u8>` maps to the `Bytes` wire kind, outer `Vec` wraps it in
//! a `List`): `BTreeMap<u8, Vec<Vec<u8>>>`.

use pyroformat::{Layout, Writer, Reader};
use std::collections::BTreeMap;

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct MapU8ToListBytes32 {
    a: BTreeMap<u8, Vec<Vec<u8>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct MapU8ToListBytes16 {
    a: BTreeMap<u8, Vec<Vec<u8>>>,
}

fn assert_pyro_traits<T: Layout + Writer + Reader>() {

}

fn main() {
    assert_pyro_traits::<MapU8ToListBytes32>();
    assert_pyro_traits::<MapU8ToListBytes16>();
}
