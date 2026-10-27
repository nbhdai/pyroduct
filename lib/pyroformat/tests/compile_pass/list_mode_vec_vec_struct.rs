//! `Vec<Vec<Struct>>` (`vec_depth == 2`) standalone field — `Matrix.data:
//! Vec<Vec<Leaf>>`. Exercises the derive's group-nesting ref-reader arm
//! (`NestedReader<{Struct}ListRefReader, N>`,
//! `docs/reader-repair-plan.md` §3.4) — **Phase 2** of that plan, not yet
//! implemented (`dynamic::refs::field_info::compute` returns `None` for any
//! struct-shaped field, so `Matrix` gets no `Ref`/`pyro_ref_reader()` yet).
//! This file is expected to fail to compile until Phase 2 lands; kept here
//! (rather than moved to `tests/compile_pass_pending`) as the marker for
//! that phase's "done when" criterion.
//!
//! `trybuild`'s `pass()` compiles *and runs* this binary, so the `assert_eq!`
//! calls below are a real round-trip check, not just a compile check.

use pyroformat::{Layout, Writer, Reader};

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct Leaf32 {
    a: i32,
    b: String,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct Leaf16 {
    a: i32,
    b: String,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct Matrix32 {
    data: Vec<Vec<Leaf32>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct Matrix16 {
    data: Vec<Vec<Leaf16>>,
}

fn assert_pyro_traits<T: Layout + Writer + Reader>() {

}

fn main() {
    assert_pyro_traits::<Leaf32>();
    assert_pyro_traits::<Leaf16>();
    assert_pyro_traits::<Matrix32>();
    assert_pyro_traits::<Matrix16>();
}
