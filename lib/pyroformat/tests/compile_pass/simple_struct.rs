use pyroformat::{Layout, Writer, Reader};

#[derive(Layout, Writer, Reader)]
#[pyro(Word = u32)]
struct SimpleStruct32 {
    id: u32,
    name: String,
}

#[derive(Layout, Writer, Reader)]
#[pyro(Word = u16)]
struct SimpleStruct16 {
    id: u32,
    name: String,
}

fn assert_pyro_traits<T: Layout + Writer + Reader>() {

}

fn main() {
    assert_pyro_traits::<SimpleStruct32>();
    assert_pyro_traits::<SimpleStruct16>();
}
