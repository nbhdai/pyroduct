//! `[T; N]` (fixed-size array) and `Option<[T; N]>` (nullable fixed-size
//! array) fields. Exercises `type_map`'s `Array`/`OptionArray` dispatch and
//! the derive's fixed-array ref-reader arms
//! (`dynamic::refs::field_info::compute`).
//!
//! `trybuild`'s `pass()` compiles *and runs* this binary, so the
//! `assert_eq!` calls below are a real round-trip check, not just a
//! compile check.

use pyroformat::{Layout, Writer, Reader};

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct WithFixedArray32 {
    id: u32,
    matrix: [u32; 4],
    maybe_matrix: Option<[i64; 3]>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct WithFixedArray16 {
    id: u32,
    matrix: [u32; 4],
    maybe_matrix: Option<[i64; 3]>,
}

fn assert_pyro_traits<T: Layout + Writer + Reader>() {

}

fn main() {
    assert_pyro_traits::<WithFixedArray32>();
    assert_pyro_traits::<WithFixedArray16>();
}
