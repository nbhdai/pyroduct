use pyroformat::{Layout, Writer, Reader};

#[derive(Layout, Writer, Reader)]
#[pyro(Word = u32)]
struct WithString32 {
    name: String,
    description: String,
}

#[derive(Layout, Writer, Reader)]
#[pyro(Word = u16)]
struct WithString16 {
    name: String,
    description: String,
}

fn assert_pyro_traits<T: Layout + Writer + Reader>() {

}

fn main() {
    assert_pyro_traits::<WithString32>();
    assert_pyro_traits::<WithString16>();
}
