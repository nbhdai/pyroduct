//! Test #[derive(Layout, Writer, Reader)] with empty struct (edge case)

use pyroformat::{Reader, Layout, Writer};

#[derive(Layout, Writer, Reader)]
#[pyro(Word = u32)]
struct Empty32 {}

#[derive(Layout, Writer, Reader)]
#[pyro(Word = u16)]
struct Empty16 {}

fn assert_pyro_traits<T: Layout + Writer + Reader>() {}

fn main() {
    assert_pyro_traits::<Empty32>();
    assert_pyro_traits::<Empty16>();
}
