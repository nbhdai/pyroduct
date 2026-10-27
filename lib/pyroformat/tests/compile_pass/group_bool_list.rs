use pyroformat::{Layout, Reader, Writer};

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GBoolInner32 {
    a: bool,
    #[pyro(allow(vec_bool))]
    b: Vec<Vec<bool>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GBoolInner16 {
    a: bool,
    #[pyro(allow(vec_bool))]
    b: Vec<Vec<bool>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GListGroupBool32 {
    a: Vec<GBoolInner32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GListGroupBool16 {
    a: Vec<GBoolInner16>,
}

fn assert_pyro_traits<T: Layout + Writer + Reader>() {}

fn main() {
    assert_pyro_traits::<GBoolInner32>();
    assert_pyro_traits::<GBoolInner16>();
    assert_pyro_traits::<GListGroupBool32>();
    assert_pyro_traits::<GListGroupBool16>();
}
