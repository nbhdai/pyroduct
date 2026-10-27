//! `[bool; N]` and `Option<[bool; N]>` fields, bare. `Vec<[bool; N]>` and
//! `Vec<Option<[bool; N]>>` already work; the bare forms fail to compile
//! under `Reader`, whose view for them is a `&BitSlice` that has no
//! `Realize<[bool; N]>`.
//!
//! `Layout, Reader` alone must be enough: the reader depends only on the
//! layout. `trybuild`'s `pass()` also runs this binary, so the round trips
//! below are real checks.

use pyroformat::{Layout, Reader, Realize, Writer};

#[derive(Layout, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct ReadOnlyBools {
    flags: [bool; 9],
    maybe: Option<[bool; 3]>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct Bools32 {
    flags: [bool; 9],
    maybe: Option<[bool; 3]>,
    absent: Option<[bool; 3]>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct Bools16 {
    flags: [bool; 9],
    maybe: Option<[bool; 3]>,
    absent: Option<[bool; 3]>,
}

fn assert_reader<T: Layout + Reader>() {}

fn main() {
    assert_reader::<ReadOnlyBools>();

    let data = Bools32 {
        flags: [true, false, true, true, false, false, true, false, true],
        maybe: Some([false, true, true]),
        absent: None,
    };
    let buf = data.encode().unwrap();
    assert_eq!(Bools32::pyro_read(&buf).unwrap().realize(), data);

    let data = Bools16 {
        flags: [false; 9],
        maybe: Some([true, true, true]),
        absent: None,
    };
    let buf = data.encode().unwrap();
    assert_eq!(Bools16::pyro_read(&buf).unwrap().realize(), data);
}
