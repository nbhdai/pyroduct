//! `Map<Vec<Vec<u8>>, Vec<Vec<u8>>>` — `List<Bytes>` on both the key and
//! value side of the same map.

use pyroformat::{Layout, Writer, Reader};
use std::collections::BTreeMap;

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct MapListBytesToListBytes32 {
    a: BTreeMap<Vec<Vec<u8>>, Vec<Vec<u8>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct MapListBytesToListBytes16 {
    a: BTreeMap<Vec<Vec<u8>>, Vec<Vec<u8>>>,
}

fn assert_pyro_traits<T: Layout + Writer + Reader>() {

}

fn main() {
    assert_pyro_traits::<MapListBytesToListBytes32>();
    assert_pyro_traits::<MapListBytesToListBytes16>();
}
