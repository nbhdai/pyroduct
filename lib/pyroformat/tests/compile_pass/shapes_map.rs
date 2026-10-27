//! Every derived struct shape from `tests/value_match/map_u32.rs`, minus
//! the golden-fixture plumbing: each must derive `Layout`, `Writer`
//! and `Reader`. Generated from the value_match file — keep in sync.

use std::collections::BTreeMap;
use pyroformat::{Layout, Reader, Writer};

fn assert_pyro_traits<T: Layout + Writer + Reader>() {}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PMapStrU32W32 {
    a: BTreeMap<String, u32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PMapStrU32W16 {
    a: BTreeMap<String, u32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PMapU8U32W32 {
    a: BTreeMap<u8, u32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PMapU8U32W16 {
    a: BTreeMap<u8, u32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PMapStrOptionU32W32 {
    a: BTreeMap<String, Option<u32>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PMapStrOptionU32W16 {
    a: BTreeMap<String, Option<u32>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PVecMapStrU32W32 {
    a: Vec<BTreeMap<String, u32>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PVecMapStrU32W16 {
    a: Vec<BTreeMap<String, u32>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[pyro(Word = u32)]
struct GMapXY32 {
    x: u32,
    y: u32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[pyro(Word = u16)]
struct GMapXY16 {
    x: u32,
    y: u32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PMapU8Group32 {
    a: BTreeMap<u8, GMapXY32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PMapU8Group16 {
    a: BTreeMap<u8, GMapXY16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PMapGroupU8W32 {
    a: BTreeMap<GMapXY32, u8>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PMapGroupU8W16 {
    a: BTreeMap<GMapXY16, u8>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PMapU8ListGroup32 {
    a: BTreeMap<u8, Vec<GMapXY32>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PMapU8ListGroup16 {
    a: BTreeMap<u8, Vec<GMapXY16>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PMapListGroupU8W32 {
    a: BTreeMap<Vec<GMapXY32>, u8>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PMapListGroupU8W16 {
    a: BTreeMap<Vec<GMapXY16>, u8>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PMapU8ListBytes32 {
    a: BTreeMap<u8, Vec<Vec<u8>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PMapU8ListBytes16 {
    a: BTreeMap<u8, Vec<Vec<u8>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PMapListBytesU8W32 {
    a: BTreeMap<Vec<Vec<u8>>, u8>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PMapListBytesU8W16 {
    a: BTreeMap<Vec<Vec<u8>>, u8>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PMapListBytesListBytes32 {
    a: BTreeMap<Vec<Vec<u8>>, Vec<Vec<u8>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PMapListBytesListBytes16 {
    a: BTreeMap<Vec<Vec<u8>>, Vec<Vec<u8>>>,
}

fn main() {
    assert_pyro_traits::<PMapStrU32W32>();
    assert_pyro_traits::<PMapStrU32W16>();
    assert_pyro_traits::<PMapU8U32W32>();
    assert_pyro_traits::<PMapU8U32W16>();
    assert_pyro_traits::<PMapStrOptionU32W32>();
    assert_pyro_traits::<PMapStrOptionU32W16>();
    assert_pyro_traits::<PVecMapStrU32W32>();
    assert_pyro_traits::<PVecMapStrU32W16>();
    assert_pyro_traits::<GMapXY32>();
    assert_pyro_traits::<GMapXY16>();
    assert_pyro_traits::<PMapU8Group32>();
    assert_pyro_traits::<PMapU8Group16>();
    assert_pyro_traits::<PMapGroupU8W32>();
    assert_pyro_traits::<PMapGroupU8W16>();
    assert_pyro_traits::<PMapU8ListGroup32>();
    assert_pyro_traits::<PMapU8ListGroup16>();
    assert_pyro_traits::<PMapListGroupU8W32>();
    assert_pyro_traits::<PMapListGroupU8W16>();
    assert_pyro_traits::<PMapU8ListBytes32>();
    assert_pyro_traits::<PMapU8ListBytes16>();
    assert_pyro_traits::<PMapListBytesU8W32>();
    assert_pyro_traits::<PMapListBytesU8W16>();
    assert_pyro_traits::<PMapListBytesListBytes32>();
    assert_pyro_traits::<PMapListBytesListBytes16>();
}
