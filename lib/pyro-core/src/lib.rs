pub mod ffi;
pub mod format;
pub mod module;
pub(crate) mod utils;

#[cfg(test)]
pub mod fmt {
    use proc_macro2::TokenStream;
    use similar::{ChangeTag, TextDiff};
    use std::fmt::Write; // Import Write for efficient string building

    pub fn format_tokens(tokens: &TokenStream) -> String {
        match syn::parse_file(&tokens.to_string()) {
            Ok(file) => prettyplease::unparse(&file),
            Err(err) => {
                println!("Parsing Error {err:?}");
                tokens.to_string()
            }
        }
    }

    pub fn assert_code_eq_token(tokens: &TokenStream, expected: &TokenStream) {
        let actual_file: syn::File = match syn::parse2(tokens.clone()) {
            Ok(actual_file) => actual_file,
            Err(err) => {
                println!("=== Bad Code: ===");
                println!("{tokens}");
                println!("=================");
                println!("Error: {err}");
                panic!("Actual string is not valid Rust code (syn::File)")
            }
        };
        let expected_file: syn::File = match syn::parse2(expected.clone()) {
            Ok(actual_file) => actual_file,
            Err(err) => {
                println!("=== Bad Code: ===");
                println!("{expected}");
                println!("=================");
                println!("Error: {err}");
                panic!("Expected string is not valid Rust code (syn::File)")
            }
        };

        let actual_str = prettyplease::unparse(&actual_file);
        let expected_str = prettyplease::unparse(&expected_file);

        if actual_str != expected_str {
            // "expected" is the original, "actual" is the new version
            let diff = TextDiff::from_lines(&expected_str, &actual_str);
            let mut diff_output = String::new();

            for change in diff.iter_all_changes() {
                let sign = match change.tag() {
                    ChangeTag::Delete => "-", // Present in expected, missing in actual
                    ChangeTag::Insert => "+", // Present in actual, missing in expected
                    ChangeTag::Equal => " ",
                };

                // Write the formatted line to the buffer
                let _ = write!(&mut diff_output, "{}{}", sign, change);
            }

            panic!(
                "Code mismatch!\n\nDIFF ((- expected) (+ actual)):\n\n{}\n",
                diff_output
            );
        }
    }
}
