use pyroformat::{Layout, Writer, Reader};
use std::collections::BTreeMap;

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PListMapU8U8W32 {
    a: Vec<BTreeMap<u8, u8>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PListMapU8U8W16 {
    a: Vec<BTreeMap<u8, u8>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GroupNest32 {
    b: Vec<Vec<PListMapU8U8W32>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GroupNest16 {
    b: Vec<Vec<PListMapU8U8W16>>,
}

fn assert_pyro_traits<T: Layout + Writer + Reader>() {

}

fn main() {
    assert_pyro_traits::<PListMapU8U8W32>();
    assert_pyro_traits::<PListMapU8U8W16>();
    assert_pyro_traits::<GroupNest32>();
    assert_pyro_traits::<GroupNest16>();
}
