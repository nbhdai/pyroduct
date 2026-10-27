use pyroformat::{Reader, Layout, Writer};

#[derive(Layout, Writer, Reader)]
#[pyro(Word = u32)]
struct WithVec32 {
    i32_vec: Vec<i32>,
    u64_vec: Vec<u64>,
    f64_vec: Vec<f64>,
    #[pyro(allow(vec_bool))]
    bool_vec: Vec<bool>,
}

#[derive(Layout, Writer, Reader)]
#[pyro(Word = u16)]
struct WithVec16 {
    i32_vec: Vec<i32>,
    u64_vec: Vec<u64>,
    f64_vec: Vec<f64>,
    #[pyro(allow(vec_bool))]
    bool_vec: Vec<bool>,
}

fn assert_pyro_traits<T: Layout + Writer + Reader>() {}

fn main() {
    assert_pyro_traits::<WithVec32>();
    assert_pyro_traits::<WithVec16>();
}
