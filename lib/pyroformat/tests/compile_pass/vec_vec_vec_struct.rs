//! `List<List<List<Group<...>>>>` — three list levels over a required group.
//!
//! The depth-3 twin of `list_mode_vec_vec_struct`, exercising the group
//! regrouping walk at a level the two-level readers don't reach.

use pyroformat::{Layout, Writer, Reader};

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct Leaf32 {
    a: i32,
    b: String,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct Leaf16 {
    a: i32,
    b: String,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct Cube32 {
    data: Vec<Vec<Vec<Leaf32>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct Cube16 {
    data: Vec<Vec<Vec<Leaf16>>>,
}

fn assert_pyro_traits<T: Layout + Writer + Reader>() {

}

fn main() {
    assert_pyro_traits::<Leaf32>();
    assert_pyro_traits::<Leaf16>();
    assert_pyro_traits::<Cube32>();
    assert_pyro_traits::<Cube16>();
}
