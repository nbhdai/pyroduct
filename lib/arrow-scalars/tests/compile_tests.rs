//! Compile-time tests using trybuild
//!
//! These tests verify that:
//! 1. Valid code compiles successfully
//! 2. Invalid code produces helpful error messages

#[test]
fn compile_pass_tests() {
    let t = trybuild::TestCases::new();
    t.pass("tests/compile_pass/*.rs");
}

#[test]
fn compile_fail_tests() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/*.rs");
}
