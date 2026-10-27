//! `pyro_optimize!`'s own cycle detection (`docs/pyro-optimize-plan.md`,
//! Step 2/M5) must fail with a clear, macro-authored `compile_error!` naming
//! the cycle — not a raw rustc `E0391` (see `tests/compile_fail/
//! recursive_schema.rs`, the per-struct-derive version of this same shape)
//! and not a silent miscompile or a hang.
use pyroformat::{Reader, Layout, Writer};

#[derive(Layout, Writer, Reader)]
struct A {
    bs: Vec<B>,
}

#[derive(Layout, Writer, Reader)]
struct B {
    a_s: Vec<A>,
}

fn main() {}
