//! Compile tests for Bridgeable derive macro

#[test]
fn bridgeable_compile_tests() {
    let t = trybuild::TestCases::new();
    t.pass("tests/compile_pass/*.rs");
    t.compile_fail("tests/compile_fail/*.rs");
}
