use pyroformat::{Layout, Writer, Reader};

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GFixedInner32 {
    a: [u8; 16],
    b: Vec<[u16; 8]>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GFixedInner16 {
    a: [u8; 16],
    b: Vec<[u16; 8]>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GListGroupFixedArrays32 {
    a: Vec<GFixedInner32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GListGroupFixedArrays16 {
    a: Vec<GFixedInner16>,
}

fn assert_pyro_traits<T: Layout + Writer + Reader>() {

}

fn main() {
    assert_pyro_traits::<GFixedInner32>();
    assert_pyro_traits::<GFixedInner16>();
    assert_pyro_traits::<GListGroupFixedArrays32>();
    assert_pyro_traits::<GListGroupFixedArrays16>();
}
