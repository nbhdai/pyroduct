//! Every derived struct shape from `tests/value_match/difficult_u32.rs`, minus
//! the golden-fixture plumbing: each must derive `Layout`, `Writer`
//! and `Reader`. Generated from the value_match file — keep in sync.

use pyroformat::{Layout, Reader, Writer};

fn assert_pyro_traits<T: Layout + Writer + Reader>() {}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PDepth4NestedU32W32 {
    a: Vec<Vec<Vec<Vec<u32>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PDepth4NestedU32W16 {
    a: Vec<Vec<Vec<Vec<u32>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PDepth5NestedU32W32 {
    a: Vec<Vec<Vec<Vec<Vec<u32>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PDepth5NestedU32W16 {
    a: Vec<Vec<Vec<Vec<Vec<u32>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PLargeU32List32 {
    a: Vec<u32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PLargeU32List16 {
    a: Vec<u32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PGroup100U8FieldsInner32 {
    f0: u8,
    f1: u8,
    f2: u8,
    f3: u8,
    f4: u8,
    f5: u8,
    f6: u8,
    f7: u8,
    f8: u8,
    f9: u8,
    f10: u8,
    f11: u8,
    f12: u8,
    f13: u8,
    f14: u8,
    f15: u8,
    f16: u8,
    f17: u8,
    f18: u8,
    f19: u8,
    f20: u8,
    f21: u8,
    f22: u8,
    f23: u8,
    f24: u8,
    f25: u8,
    f26: u8,
    f27: u8,
    f28: u8,
    f29: u8,
    f30: u8,
    f31: u8,
    f32: u8,
    f33: u8,
    f34: u8,
    f35: u8,
    f36: u8,
    f37: u8,
    f38: u8,
    f39: u8,
    f40: u8,
    f41: u8,
    f42: u8,
    f43: u8,
    f44: u8,
    f45: u8,
    f46: u8,
    f47: u8,
    f48: u8,
    f49: u8,
    f50: u8,
    f51: u8,
    f52: u8,
    f53: u8,
    f54: u8,
    f55: u8,
    f56: u8,
    f57: u8,
    f58: u8,
    f59: u8,
    f60: u8,
    f61: u8,
    f62: u8,
    f63: u8,
    f64: u8,
    f65: u8,
    f66: u8,
    f67: u8,
    f68: u8,
    f69: u8,
    f70: u8,
    f71: u8,
    f72: u8,
    f73: u8,
    f74: u8,
    f75: u8,
    f76: u8,
    f77: u8,
    f78: u8,
    f79: u8,
    f80: u8,
    f81: u8,
    f82: u8,
    f83: u8,
    f84: u8,
    f85: u8,
    f86: u8,
    f87: u8,
    f88: u8,
    f89: u8,
    f90: u8,
    f91: u8,
    f92: u8,
    f93: u8,
    f94: u8,
    f95: u8,
    f96: u8,
    f97: u8,
    f98: u8,
    f99: u8,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PGroup100U8FieldsInner16 {
    f0: u8,
    f1: u8,
    f2: u8,
    f3: u8,
    f4: u8,
    f5: u8,
    f6: u8,
    f7: u8,
    f8: u8,
    f9: u8,
    f10: u8,
    f11: u8,
    f12: u8,
    f13: u8,
    f14: u8,
    f15: u8,
    f16: u8,
    f17: u8,
    f18: u8,
    f19: u8,
    f20: u8,
    f21: u8,
    f22: u8,
    f23: u8,
    f24: u8,
    f25: u8,
    f26: u8,
    f27: u8,
    f28: u8,
    f29: u8,
    f30: u8,
    f31: u8,
    f32: u8,
    f33: u8,
    f34: u8,
    f35: u8,
    f36: u8,
    f37: u8,
    f38: u8,
    f39: u8,
    f40: u8,
    f41: u8,
    f42: u8,
    f43: u8,
    f44: u8,
    f45: u8,
    f46: u8,
    f47: u8,
    f48: u8,
    f49: u8,
    f50: u8,
    f51: u8,
    f52: u8,
    f53: u8,
    f54: u8,
    f55: u8,
    f56: u8,
    f57: u8,
    f58: u8,
    f59: u8,
    f60: u8,
    f61: u8,
    f62: u8,
    f63: u8,
    f64: u8,
    f65: u8,
    f66: u8,
    f67: u8,
    f68: u8,
    f69: u8,
    f70: u8,
    f71: u8,
    f72: u8,
    f73: u8,
    f74: u8,
    f75: u8,
    f76: u8,
    f77: u8,
    f78: u8,
    f79: u8,
    f80: u8,
    f81: u8,
    f82: u8,
    f83: u8,
    f84: u8,
    f85: u8,
    f86: u8,
    f87: u8,
    f88: u8,
    f89: u8,
    f90: u8,
    f91: u8,
    f92: u8,
    f93: u8,
    f94: u8,
    f95: u8,
    f96: u8,
    f97: u8,
    f98: u8,
    f99: u8,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PGroup100U8Fields32 {
    a: PGroup100U8FieldsInner32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PGroup100U8Fields16 {
    a: PGroup100U8FieldsInner16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PStrExtreme32 {
    a: String,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PStrExtreme16 {
    a: String,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PBytesExtreme32 {
    a: Vec<u8>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PBytesExtreme16 {
    a: Vec<u8>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNullabilityListOptionU32W32 {
    a: Vec<Option<u32>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNullabilityListOptionU32W16 {
    a: Vec<Option<u32>>,
}

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

fn main() {
    assert_pyro_traits::<PDepth4NestedU32W32>();
    assert_pyro_traits::<PDepth4NestedU32W16>();
    assert_pyro_traits::<PDepth5NestedU32W32>();
    assert_pyro_traits::<PDepth5NestedU32W16>();
    assert_pyro_traits::<PLargeU32List32>();
    assert_pyro_traits::<PLargeU32List16>();
    assert_pyro_traits::<PGroup100U8FieldsInner32>();
    assert_pyro_traits::<PGroup100U8FieldsInner16>();
    assert_pyro_traits::<PGroup100U8Fields32>();
    assert_pyro_traits::<PGroup100U8Fields16>();
    assert_pyro_traits::<PStrExtreme32>();
    assert_pyro_traits::<PStrExtreme16>();
    assert_pyro_traits::<PBytesExtreme32>();
    assert_pyro_traits::<PBytesExtreme16>();
    assert_pyro_traits::<PNullabilityListOptionU32W32>();
    assert_pyro_traits::<PNullabilityListOptionU32W16>();
    assert_pyro_traits::<PGroup20OptionBoolInner32>();
    assert_pyro_traits::<PGroup20OptionBoolInner16>();
    assert_pyro_traits::<PGroup20OptionBool32>();
    assert_pyro_traits::<PGroup20OptionBool16>();
}
