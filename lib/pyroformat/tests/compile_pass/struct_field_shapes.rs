use pyroformat::{Layout, Writer, Reader};

#[derive(Layout, Writer, Reader, Debug, Clone, PartialEq)]
#[pyro(Word = u32)]
struct Leaf32 {
    a: i32,
    b: String,
}

#[derive(Layout, Writer, Reader, Debug, Clone, PartialEq)]
#[pyro(Word = u16)]
struct Leaf16 {
    a: i32,
    b: String,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct DirectLeaf32 {
    leaf: Leaf32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct DirectLeaf16 {
    leaf: Leaf16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct DeepVecLeaf32 {
    leaves: Vec<Vec<Vec<Leaf32>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct DeepVecLeaf16 {
    leaves: Vec<Vec<Vec<Leaf16>>>,
}

fn assert_pyro_traits<T: Layout + Writer + Reader>() {

}

fn main() {
    assert_pyro_traits::<Leaf32>();
    assert_pyro_traits::<Leaf16>();
    assert_pyro_traits::<DirectLeaf32>();
    assert_pyro_traits::<DirectLeaf16>();
    assert_pyro_traits::<DeepVecLeaf32>();
    assert_pyro_traits::<DeepVecLeaf16>();
}
