use pyroformat::{Layout, Writer, Reader};

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct WithTags32 {
    name: String,
    tags: Vec<String>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct WithTags16 {
    name: String,
    tags: Vec<String>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct Team32 {
    people: Vec<WithTags32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct Team16 {
    people: Vec<WithTags16>,
}

fn assert_pyro_traits<T: Layout + Writer + Reader>() {

}

fn main() {
    assert_pyro_traits::<WithTags32>();
    assert_pyro_traits::<WithTags16>();
    assert_pyro_traits::<Team32>();
    assert_pyro_traits::<Team16>();
}
