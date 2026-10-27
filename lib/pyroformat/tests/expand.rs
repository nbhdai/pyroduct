//! Snapshot tests of what the derives expand to: each `tests/expand/*.rs`
//! is compared with its `*.expanded.rs`.
//!
//! Needs `cargo-expand` and a nightly toolchain. A missing `.expanded.rs`
//! is written on the first run; regenerate them all with
//! `MACROTEST=overwrite cargo test --test expand`.

#[test]
fn expand() {
    macrotest::expand("tests/expand/*.rs");
}
