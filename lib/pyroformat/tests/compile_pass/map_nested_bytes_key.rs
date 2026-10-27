//! `Map<Vec<Vec<u8>>, ...>` — the mirror of `map_nested_bytes_value`, a
//! `List<Bytes>` map key instead: `BTreeMap<Vec<Vec<u8>>, u8>`.

use pyroformat::{Layout, Writer, Reader};
use std::collections::BTreeMap;

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct MapListBytesToU8W32 {
    a: BTreeMap<Vec<Vec<u8>>, u8>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct MapListBytesToU8W16 {
    a: BTreeMap<Vec<Vec<u8>>, u8>,
}

fn assert_pyro_traits<T: Layout + Writer + Reader>() {

}

fn main() {
    assert_pyro_traits::<MapListBytesToU8W32>();
    assert_pyro_traits::<MapListBytesToU8W16>();
}
