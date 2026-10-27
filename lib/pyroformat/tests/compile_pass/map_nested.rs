use pyroformat::{Layout, Writer, Reader};
use std::collections::BTreeMap;

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PListListMapU8U8W32 {
    a: Vec<Vec<BTreeMap<u8, u8>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PListListMapU8U8W16 {
    a: Vec<Vec<BTreeMap<u8, u8>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PListListListMapU8U8W32 {
    a: Vec<Vec<Vec<BTreeMap<u8, u8>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PListListListMapU8U8W16 {
    a: Vec<Vec<Vec<BTreeMap<u8, u8>>>>,
}

/// Forces the compiler to check that `T` implements `Layout, Writer, Reader` (which in
/// turn requires `WriterPrep: PyroPrep` and `Writer<'a>: PyroWriter`, per
/// the supertrait bounds on the associated types in `Layout, Writer, Reader`) and
/// `PyroRead`.

fn assert_pyro_traits<T: Layout + Writer + Reader>() {

}

fn main() {
    assert_pyro_traits::<PListListMapU8U8W32>();
    assert_pyro_traits::<PListListMapU8U8W16>();
    assert_pyro_traits::<PListListListMapU8U8W32>();
    assert_pyro_traits::<PListListListMapU8U8W16>();
}
