//! Test #[derive(Layout, Writer, Reader)] with Option fields

use pyroformat::{Layout, Writer, Reader};

#[derive(Layout, Writer, Reader)]
#[pyro(Word = u32)]
struct WithOption32 {
    required: i32,
    optional_num: Option<i32>,
    optional_str: String,
}

#[derive(Layout, Writer, Reader)]
#[pyro(Word = u16)]
struct WithOption16 {
    required: i32,
    optional_num: Option<i32>,
    optional_str: String,
}

fn assert_pyro_traits<T: Layout + Writer + Reader>() {

}

fn main() {
    assert_pyro_traits::<WithOption32>();
    assert_pyro_traits::<WithOption16>();
}
