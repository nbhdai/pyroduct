//! `#[derive(Layout, Writer, Reader)]` on a struct with a generic type parameter should
//! fail cleanly (not support the feature, but not silently generate broken
//! code referencing a nonexistent `TPrep`/`TWriter` either).
//!
//! Unlike `tests/compile_fail/generic.rs`/`multi_generic.rs` (which predate
//! this and test an unrelated `#[magma]`/`pyroduct` macro that was never
//! actually part of this crate), this exercises the real
//! `#[derive(Layout, Writer, Reader)]` path.

use pyroformat::{Layout, Writer, Reader};

#[derive(Layout, Writer, Reader)]
struct Wrapper<T> {
    name: String,
    value: T,
}

fn main() {}
