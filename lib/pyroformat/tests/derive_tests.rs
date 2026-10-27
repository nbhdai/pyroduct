#[test]
fn compile_tests() {
    let t = trybuild::TestCases::new();
    t.pass("tests/compile_pass/*.rs");
    t.compile_fail("tests/compile_fail/*.rs");
}

#[test]
fn unsealed_buffers_cannot_view_or_cross_ffi() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail_io/*.rs");
}
