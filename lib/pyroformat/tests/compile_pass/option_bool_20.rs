use pyroformat::{Layout, Writer, Reader};

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PGroup20OptionBoolInner32 {
    f0: Option<bool>,
    f1: Option<bool>,
    f2: Option<bool>,
    f3: Option<bool>,
    f4: Option<bool>,
    f5: Option<bool>,
    f6: Option<bool>,
    f7: Option<bool>,
    f8: Option<bool>,
    f9: Option<bool>,
    f10: Option<bool>,
    f11: Option<bool>,
    f12: Option<bool>,
    f13: Option<bool>,
    f14: Option<bool>,
    f15: Option<bool>,
    f16: Option<bool>,
    f17: Option<bool>,
    f18: Option<bool>,
    f19: Option<bool>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PGroup20OptionBoolInner16 {
    f0: Option<bool>,
    f1: Option<bool>,
    f2: Option<bool>,
    f3: Option<bool>,
    f4: Option<bool>,
    f5: Option<bool>,
    f6: Option<bool>,
    f7: Option<bool>,
    f8: Option<bool>,
    f9: Option<bool>,
    f10: Option<bool>,
    f11: Option<bool>,
    f12: Option<bool>,
    f13: Option<bool>,
    f14: Option<bool>,
    f15: Option<bool>,
    f16: Option<bool>,
    f17: Option<bool>,
    f18: Option<bool>,
    f19: Option<bool>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PGroup20OptionBool32 {
    a: PGroup20OptionBoolInner32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PGroup20OptionBool16 {
    a: PGroup20OptionBoolInner16,
}


fn assert_pyro_traits<T: Layout + Writer + Reader>() {

}

fn main() {
    assert_pyro_traits::<PGroup20OptionBoolInner32>();
    assert_pyro_traits::<PGroup20OptionBoolInner16>();
    assert_pyro_traits::<PGroup20OptionBool32>();
    assert_pyro_traits::<PGroup20OptionBool16>();
}
