//! Struct-vs-value match for `src/wire/tests/basic_u32.rs` — `Str`/`Bytes` at
//! the top level. See `tests/value_match/primitive_u32.rs`'s module doc for the
//! shared rationale.

use pyroformat::wire::tests::basic_u32::*;

use crate::support::read_back::assert_roundtrip;
use pyroformat::{Layout, Reader, Writer};

macro_rules! value_match_basic_test {
    ($test_name:ident, $struct_ty:ident, $field:expr, $buf_const:expr, $inodes_const:expr) => {
        #[test]
        fn $test_name() {
            let data = $struct_ty { a: $field };
            assert_roundtrip(&data, &$buf_const, &$inodes_const);
        }
    };
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PStr {
    a: String,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PBytes {
    a: Vec<u8>,
}

value_match_basic_test!(
    value_match_str_basic,
    PStr,
    "hello".to_string(),
    STR_BASIC,
    STR_BASIC_INODES
);
value_match_basic_test!(
    value_match_str_empty,
    PStr,
    "".to_string(),
    STR_EMPTY,
    STR_EMPTY_INODES
);
value_match_basic_test!(
    value_match_str_embedded_nul,
    PStr,
    "foo\0bar".to_string(),
    STR_EMBEDDED_NUL,
    STR_EMBEDDED_NUL_INODES
);
value_match_basic_test!(
    value_match_str_emoji,
    PStr,
    "🔥".to_string(),
    STR_EMOJI,
    STR_EMOJI_INODES
);
value_match_basic_test!(
    value_match_str_combining,
    PStr,
    "e\u{0301}".to_string(),
    STR_COMBINING,
    STR_COMBINING_INODES
);
value_match_basic_test!(
    value_match_str_rtl,
    PStr,
    "אבג".to_string(),
    STR_RTL,
    STR_RTL_INODES
);
value_match_basic_test!(
    value_match_str_zwj_sequence,
    PStr,
    "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}".to_string(),
    STR_ZWJ_SEQUENCE,
    STR_ZWJ_SEQUENCE_INODES
);

value_match_basic_test!(
    value_match_bytes_basic,
    PBytes,
    vec![1u8, 2, 3, 4, 5],
    BYTES_BASIC,
    BYTES_BASIC_INODES
);
value_match_basic_test!(
    value_match_bytes_empty,
    PBytes,
    Vec::<u8>::new(),
    BYTES_EMPTY,
    BYTES_EMPTY_INODES
);
