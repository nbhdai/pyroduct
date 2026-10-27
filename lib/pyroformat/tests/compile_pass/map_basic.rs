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

/// Forces the compiler to check that `T` implements `Layout, Writer, Reader` (which in
/// turn requires `WriterPrep: PyroPrep` and `Writer<'a>: PyroWriter`, per
/// the supertrait bounds on the associated types in `Layout, Writer, Reader`) and
/// `PyroRead`.

fn assert_pyro_traits<T: Layout + Writer + Reader>() {

}

fn main() {
    assert_pyro_traits::<PListMapU8U8W32>();
    assert_pyro_traits::<PListMapU8U8W16>();
}
