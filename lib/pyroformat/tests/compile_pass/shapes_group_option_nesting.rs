//! Every derived struct shape from `tests/value_match/group_option_nesting_u32.rs`, minus
//! the golden-fixture plumbing: each must derive `Layout`, `Writer`
//! and `Reader`. Generated from the value_match file — keep in sync.

use pyroformat::{Layout, Reader, Writer};

fn assert_pyro_traits<T: Layout + Writer + Reader>() {}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct LOptGroupY32 {
    y: u8,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct LOptGroupY16 {
    y: u8,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PListOptGroupY32 {
    a: Vec<Option<LOptGroupY32>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PListOptGroupY16 {
    a: Vec<Option<LOptGroupY16>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GroupOptPrim32 {
    x: Option<i8>,
    y: f32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GroupOptPrim16 {
    x: Option<i8>,
    y: f32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PListGroupOptPrim32 {
    a: Vec<GroupOptPrim32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PListGroupOptPrim16 {
    a: Vec<GroupOptPrim16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GroupOptGroupInner32 {
    x: Option<i8>,
    y: f32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GroupOptGroupInner16 {
    x: Option<i8>,
    y: f32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GroupOptGroupElem32 {
    a: u8,
    c: Option<GroupOptGroupInner32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GroupOptGroupElem16 {
    a: u8,
    c: Option<GroupOptGroupInner16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PListGroupOptionGroup32 {
    a: Vec<GroupOptGroupElem32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PListGroupOptionGroup16 {
    a: Vec<GroupOptGroupElem16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct DoubleNestedY32 {
    p: u16,
    q: Option<bool>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct DoubleNestedY16 {
    p: u16,
    q: Option<bool>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct DoubleNestedB32 {
    x: Option<i8>,
    y: Option<DoubleNestedY32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct DoubleNestedB16 {
    x: Option<i8>,
    y: Option<DoubleNestedY16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct DoubleNestedElem32 {
    a: u8,
    b: Option<DoubleNestedB32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct DoubleNestedElem16 {
    a: u8,
    b: Option<DoubleNestedB16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PListGroupDoubleNestedOptionGroup32 {
    a: Vec<DoubleNestedElem32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PListGroupDoubleNestedOptionGroup16 {
    a: Vec<DoubleNestedElem16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GroupOptF64BoolElem32 {
    t: Option<f64>,
    u: bool,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GroupOptF64BoolElem16 {
    t: Option<f64>,
    u: bool,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PListGroupOptionF64Bool32 {
    a: Vec<Option<GroupOptF64BoolElem32>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PListGroupOptionF64Bool16 {
    a: Vec<Option<GroupOptF64BoolElem16>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GroupOptU8Elem32 {
    x: Option<u8>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GroupOptU8Elem16 {
    x: Option<u8>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PEmptyListGroupOption32 {
    a: Vec<GroupOptU8Elem32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PEmptyListGroupOption16 {
    a: Vec<GroupOptU8Elem16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GroupOptI8OptF32W32 {
    x: Option<i8>,
    y: Option<f32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GroupOptI8OptF32W16 {
    x: Option<i8>,
    y: Option<f32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PDoubleListGroupOption32 {
    a: Vec<Vec<Option<GroupOptI8OptF32W32>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PDoubleListGroupOption16 {
    a: Vec<Vec<Option<GroupOptI8OptF32W16>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GroupOptU8U16Elem32 {
    a: Option<u8>,
    b: Option<u16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GroupOptU8U16Elem16 {
    a: Option<u8>,
    b: Option<u16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PTripleListGroupOption32 {
    a: Vec<Vec<Vec<Option<GroupOptU8U16Elem32>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PTripleListGroupOption16 {
    a: Vec<Vec<Vec<Option<GroupOptU8U16Elem16>>>>,
}

fn main() {
    assert_pyro_traits::<LOptGroupY32>();
    assert_pyro_traits::<LOptGroupY16>();
    assert_pyro_traits::<PListOptGroupY32>();
    assert_pyro_traits::<PListOptGroupY16>();
    assert_pyro_traits::<GroupOptPrim32>();
    assert_pyro_traits::<GroupOptPrim16>();
    assert_pyro_traits::<PListGroupOptPrim32>();
    assert_pyro_traits::<PListGroupOptPrim16>();
    assert_pyro_traits::<GroupOptGroupInner32>();
    assert_pyro_traits::<GroupOptGroupInner16>();
    assert_pyro_traits::<GroupOptGroupElem32>();
    assert_pyro_traits::<GroupOptGroupElem16>();
    assert_pyro_traits::<PListGroupOptionGroup32>();
    assert_pyro_traits::<PListGroupOptionGroup16>();
    assert_pyro_traits::<DoubleNestedY32>();
    assert_pyro_traits::<DoubleNestedY16>();
    assert_pyro_traits::<DoubleNestedB32>();
    assert_pyro_traits::<DoubleNestedB16>();
    assert_pyro_traits::<DoubleNestedElem32>();
    assert_pyro_traits::<DoubleNestedElem16>();
    assert_pyro_traits::<PListGroupDoubleNestedOptionGroup32>();
    assert_pyro_traits::<PListGroupDoubleNestedOptionGroup16>();
    assert_pyro_traits::<GroupOptF64BoolElem32>();
    assert_pyro_traits::<GroupOptF64BoolElem16>();
    assert_pyro_traits::<PListGroupOptionF64Bool32>();
    assert_pyro_traits::<PListGroupOptionF64Bool16>();
    assert_pyro_traits::<GroupOptU8Elem32>();
    assert_pyro_traits::<GroupOptU8Elem16>();
    assert_pyro_traits::<PEmptyListGroupOption32>();
    assert_pyro_traits::<PEmptyListGroupOption16>();
    assert_pyro_traits::<GroupOptI8OptF32W32>();
    assert_pyro_traits::<GroupOptI8OptF32W16>();
    assert_pyro_traits::<PDoubleListGroupOption32>();
    assert_pyro_traits::<PDoubleListGroupOption16>();
    assert_pyro_traits::<GroupOptU8U16Elem32>();
    assert_pyro_traits::<GroupOptU8U16Elem16>();
    assert_pyro_traits::<PTripleListGroupOption32>();
    assert_pyro_traits::<PTripleListGroupOption16>();
}
