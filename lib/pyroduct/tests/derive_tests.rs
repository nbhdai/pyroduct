#[test]
fn magma_compile_tests() {
    let t = trybuild::TestCases::new();
    t.pass("tests/compile_pass/magma/*.rs");
    t.compile_fail("tests/compile_fail/magma/*.rs");
}

#[test]
fn deep_ref_compile_tests() {
    let t = trybuild::TestCases::new();
    t.pass("tests/compile_pass/deep_ref/*.rs");
    t.compile_fail("tests/compile_fail/deep_ref/*.rs");
}

#[test]
fn deep_ref_expansion_tests() {
    macrotest::expand("tests/expand/deep_ref/*.rs");
}

#[test]
fn from_row_compile_tests() {
    let t = trybuild::TestCases::new();
    t.pass("tests/compile_pass/from_row/*.rs");
    t.compile_fail("tests/compile_fail/from_row/*.rs");
}

#[test]
fn from_row_expansion_tests() {
    macrotest::expand("tests/expand/from_row/*.rs");
}

#[test]
fn to_row_compile_tests() {
    let t = trybuild::TestCases::new();
    t.pass("tests/compile_pass/to_row/*.rs");
    t.compile_fail("tests/compile_fail/to_row/*.rs");
}

#[test]
fn to_row_expansion_tests() {
    macrotest::expand("tests/expand/to_row/*.rs");
}

#[test]
fn document_compile_tests() {
    let t = trybuild::TestCases::new();
    t.pass("tests/compile_pass/document/*.rs");
    t.compile_fail("tests/compile_fail/document/*.rs");
}

#[test]
fn document_expansion_tests() {
    macrotest::expand("tests/expand/document/*.rs");
}

#[test]
fn library_compile_tests() {
    let t = trybuild::TestCases::new();
    t.pass("tests/compile_pass/library/*.rs");
    t.compile_fail("tests/compile_fail/library/*.rs");
}

#[test]
fn capability_compile_tests() {
    let t = trybuild::TestCases::new();
    t.pass("tests/compile_pass/capability/*.rs");
    t.compile_fail("tests/compile_fail/capability/*.rs");
}

#[test]
fn module_compile_tests() {
    let t = trybuild::TestCases::new();
    t.pass("tests/compile_pass/module/*.rs");
    t.compile_fail("tests/compile_fail/module/*.rs");
}

#[test]
fn module_expansion_tests() {
    macrotest::expand("tests/expand/module/*.rs");
}
