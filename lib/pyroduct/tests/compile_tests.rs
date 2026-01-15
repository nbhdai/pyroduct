#[test]
fn capability_function_compile_tests() {
    let t = trybuild::TestCases::new();
    t.pass("tests/capability_function/compile_pass/*.rs");
    t.compile_fail("tests/capability_function/compile_fail/*.rs");
}

#[test]
fn capability_server_compile_tests() {
    let t = trybuild::TestCases::new();
    t.pass("tests/capability_server/compile_pass/*.rs");
    t.compile_fail("tests/capability_server/compile_fail/*.rs");
}

#[test]
fn module_compile_tests() {
    let t = trybuild::TestCases::new();
    t.pass("tests/module/compile_pass/*.rs");
    t.compile_fail("tests/module/compile_fail/*.rs");
}
