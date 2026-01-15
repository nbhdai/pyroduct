//! Tests for verifying macro expansions
//!
//! These tests capture snapshots of the generated code

#[test]
fn expansion_tests() {
    macrotest::expand("tests/expand/*.rs");
}
