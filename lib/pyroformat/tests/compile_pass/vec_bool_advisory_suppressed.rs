#![deny(warnings)]

use pyroformat::{Layout, Writer, Reader};

#[derive(Layout, Writer, Reader)]
#[pyro(Word = u32)]
struct WithSuppressedVecBool32 {
    #[pyro(allow(vec_bool))]
    flags: Vec<bool>,
}

#[derive(Layout, Writer, Reader)]
#[pyro(Word = u16)]
struct WithSuppressedVecBool16 {
    #[pyro(allow(vec_bool))]
    flags: Vec<bool>,
}

fn assert_pyro_traits<T: Layout + Writer + Reader>() {

}

fn main() {
    assert_pyro_traits::<WithSuppressedVecBool32>();
    assert_pyro_traits::<WithSuppressedVecBool16>();
}
