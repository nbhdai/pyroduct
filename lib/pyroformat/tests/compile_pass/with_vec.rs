use pyroformat::{Layout, Writer, Reader};

#[derive(Layout, Writer, Reader)]
#[pyro(Word = u32)]
struct DataContainer32 {
    items: Vec<u8>,
    labels: Vec<String>,
}

#[derive(Layout, Writer, Reader)]
#[pyro(Word = u16)]
struct DataContainer16 {
    items: Vec<u8>,
    labels: Vec<String>,
}

fn assert_pyro_traits<T: Layout + Writer + Reader>() {

}

fn main() {
    assert_pyro_traits::<DataContainer32>();
    assert_pyro_traits::<DataContainer16>();
}
