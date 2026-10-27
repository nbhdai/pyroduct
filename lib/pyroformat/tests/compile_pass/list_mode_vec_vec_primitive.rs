//! `Vec<Vec<T>>` (numeric) fields inside a struct that's itself read as one
//! row among many (list mode) — `NumTeam.members: Vec<WithNumbers>`.
//! Exercises `dynamic::refs::field_info::compute_list_mode`'s
//! `vec_depth >= 2` numeric arm (`{T}NestedReader<N>`).
//!
//! The `Vec<Vec<u8>>`-as-`Bytes` case this file used to also cover is out of
//! Phase 0/1 scope (`dynamic::refs::field_info::compute`'s module doc) — the
//! `Bytes` leaf's list-mode ref-reader arm is future work, not exercised
//! here.
//!
//! `trybuild`'s `pass()` compiles *and runs* this binary, so the `assert_eq!`
//! calls below are a real round-trip check, not just a compile check.

use pyroformat::{Layout, Writer, Reader};

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct WithNumbers32 {
    id: u32,
    matrix: Vec<Vec<i64>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct WithNumbers16 {
    id: u32,
    matrix: Vec<Vec<i64>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct NumTeam32 {
    members: Vec<WithNumbers32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct NumTeam16 {
    members: Vec<WithNumbers16>,
}

fn assert_pyro_traits<T: Layout + Writer + Reader>() {

}

fn main() {
    assert_pyro_traits::<WithNumbers32>();
    assert_pyro_traits::<WithNumbers16>();
    assert_pyro_traits::<NumTeam32>();
    assert_pyro_traits::<NumTeam16>();
}
