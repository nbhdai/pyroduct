//! A mutually-recursive schema must fail to compile with a genuine
//! const-eval cycle, not silently produce garbage or hang the compiler.
//!
//! The const-layout plan (`docs/const-layout-plan.md`, "Known limits")
//! describes this via `Box<T>` fields (`struct A { b: Box<B> }` / `struct B
//! { a: Box<A> }`), but `#[derive(Layout, Writer, Reader)]` already rejects `Box<T>`
//! fields outright (see `tests/compile_fail/box_field.rs`) — the derive
//! never gets far enough to build a const table for those. `Vec<T>` is the
//! shape that's actually supported for nested structs (see
//! `VecOfGroup`/`Point` in `tests/value_match/golden_layout.rs`) and, unlike `Box`,
//! doesn't make the struct itself infinitely-sized (`Vec<T>` has a fixed
//! size regardless of `T`), so `A`/`B` here are valid, accepted Rust types.
//! What's *not* resolvable is the const table: `A`'s `Layout::RAW_LEN`
//! is defined as `1 + <B as Layout>::RAW_LEN`, and `B`'s as
//! `1 + <A as Layout>::RAW_LEN` — a direct mutual const dependency
//! cycle, which rustc rejects with `E0391` rather than looping forever.
//! The flattened name/doc tables (`Layout::FLAT_LEN`) are defined the
//! same way and so hit the same cycle, which is why `E0391` is reported
//! twice.

use pyroformat::{Layout, Writer, Reader};

#[derive(Layout, Writer, Reader)]
struct A {
    bs: Vec<B>,
}

#[derive(Layout, Writer, Reader)]
struct B {
    a_s: Vec<A>,
}

fn main() {}
