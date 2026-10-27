use pyroformat::{Layout, Writer, Reader};

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GroupOptF64BoolElem32 {
    t: Option<f64>,
    u: bool,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GroupOptF64BoolElem16 {
    t: Option<f64>,
    u: bool,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PListGroupOptionF64Bool32 {
    a: Vec<GroupOptF64BoolElem32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PListGroupOptionF64Bool16 {
    a: Vec<GroupOptF64BoolElem16>,
}

fn assert_pyro_traits<T: Layout + Writer + Reader>() {

}

fn main() {
    assert_pyro_traits::<GroupOptF64BoolElem32>();
    assert_pyro_traits::<GroupOptF64BoolElem16>();
    assert_pyro_traits::<PListGroupOptionF64Bool32>();
    assert_pyro_traits::<PListGroupOptionF64Bool16>();
}
