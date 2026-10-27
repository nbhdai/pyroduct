#[cfg(test)]
use pyro_schema::human::{PyroField, PyroSchema, PyroType};

#[cfg(test)]
use crate::{PyroValue, wire::tests::support::run_list_wrapped_roundtrip_u32};

use pyro_schema::{Inode, KindMeta, WireKind};

// `Str`/`Bytes` at the top level (`nesting: 0`) — the non-primitive
// counterpart to `primitive.rs`. Unlike a plain scalar (whose whole value
// lives inline at `data_offset`), `Str`/`Bytes` are "wide": even a bare,
// non-list field always carries its own inline `(data_ptr: u32, data_len:
// u32)` descriptor pair pointing into a payload region right after the
// fixed region, rather than storing its bytes inline — see
// `StringWriter`/`StringReader` in `src/wire/writers/slices.rs` /
// `src/wire/readers.rs`. Since there's only ever one field here, that
// descriptor always sits at `data_offset: 0`; `expected_buf` is always
// `[8, data_len, ...payload]` — 8 because the descriptor itself is 8 bytes,
// so the payload always starts right after it.

pub const STR_BASIC: [u8; 13] = [
    8, 0, 0, 0, // data pointer
    5, 0, 0, 0, // data length
    104, 101, 108, 108, 111, // "hello"
];
pub const STR_BASIC_INODES: [Inode; 1] = [Inode {
    kind: WireKind::Str,
    kind_meta: KindMeta::Primitive {
        nesting: 0,
        data_offset: 0,
        topology_offset: 0,
        fixed_size: 0,
        nullable_bit_index: 0,
        group: false,
        group_nest: false,
    },
}];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_str_basic() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::Str,
        }],
    };

    let data = PyroValue::Str("hello".into());

    run_list_wrapped_roundtrip_u32(schema, data, &STR_BASIC, &STR_BASIC_INODES);
}

pub const STR_EMPTY: [u8; 8] = [
    8, 0, 0, 0, // data pointer
    0, 0, 0, 0, // data length — no payload bytes follow
];
pub const STR_EMPTY_INODES: [Inode; 1] = [Inode {
    kind: WireKind::Str,
    kind_meta: KindMeta::Primitive {
        nesting: 0,
        data_offset: 0,
        topology_offset: 0,
        fixed_size: 0,
        nullable_bit_index: 0,
        group: false,
        group_nest: false,
    },
}];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_str_empty() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::Str,
        }],
    };

    let data = PyroValue::Str("".into());

    run_list_wrapped_roundtrip_u32(schema, data, &STR_EMPTY, &STR_EMPTY_INODES);
}

// The wire format's `Str` is length-prefixed, not C-string-terminated,
// so an embedded `\0` byte must round-trip as an ordinary payload byte
// rather than truncating the string.
pub const STR_EMBEDDED_NUL: [u8; 15] = [
    8, 0, 0, 0, // data pointer
    7, 0, 0, 0, // data length — 7 bytes, not the 3 a C-string scan would see
    102, 111, 111, 0, 98, 97, 114, // "foo\0bar", embedded NUL included
];
pub const STR_EMBEDDED_NUL_INODES: [Inode; 1] = [Inode {
    kind: WireKind::Str,
    kind_meta: KindMeta::Primitive {
        nesting: 0,
        data_offset: 0,
        topology_offset: 0,
        fixed_size: 0,
        nullable_bit_index: 0,
        group: false,
        group_nest: false,
    },
}];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_str_embedded_nul() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::Str,
        }],
    };

    let data = PyroValue::Str("foo\0bar".into());

    run_list_wrapped_roundtrip_u32(schema, data, &STR_EMBEDDED_NUL, &STR_EMBEDDED_NUL_INODES);
}

// `Str`'s payload is raw UTF-8 bytes, not UTF-16 code units or Unicode
// scalar values, so byte length (the wire `data_len`) and "character
// count" as a human would read it routinely diverge. Each case below
// picks a string whose UTF-8 encoding stresses that in a different way;
// `expected_buf` payload bytes were hand-computed independently via
// Python's `str.encode("utf-8")`, not derived from the writer.

// A codepoint outside the Basic Multilingual Plane: encodes to 4 UTF-8
// bytes (vs. 1-3 for BMP codepoints), and would be a UTF-16 surrogate
// pair in formats that use UTF-16 — irrelevant here since the wire format
// is UTF-8 throughout, which is exactly what this case pins down.
pub const STR_EMOJI: [u8; 12] = [
    8, 0, 0, 0, // data pointer
    4, 0, 0, 0, // data length
    240, 159, 148, 165, // "🔥" (U+1F525), 4 UTF-8 bytes for 1 codepoint
];
pub const STR_EMOJI_INODES: [Inode; 1] = [Inode {
    kind: WireKind::Str,
    kind_meta: KindMeta::Primitive {
        nesting: 0,
        data_offset: 0,
        topology_offset: 0,
        fixed_size: 0,
        nullable_bit_index: 0,
        group: false,
        group_nest: false,
    },
}];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_str_emoji() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::Str,
        }],
    };

    let data = PyroValue::Str("🔥".into());

    run_list_wrapped_roundtrip_u32(schema, data, &STR_EMOJI, &STR_EMOJI_INODES);
}

// A base letter followed by a combining diacritic (two codepoints, one
// visual grapheme): "é" spelled as `e` + U+0301 COMBINING ACUTE ACCENT,
// rather than the single precomposed U+00E9 codepoint. Nothing in the
// wire format normalizes Unicode, so the two-codepoint spelling must
// survive the roundtrip byte-for-byte instead of collapsing to the
// precomposed form.
pub const STR_COMBINING: [u8; 11] = [
    8, 0, 0, 0, // data pointer
    3, 0, 0, 0, // data length
    101, 204, 129, // "e" + U+0301 COMBINING ACUTE ACCENT, NOT precomposed "é" (U+00E9)
];
pub const STR_COMBINING_INODES: [Inode; 1] = [Inode {
    kind: WireKind::Str,
    kind_meta: KindMeta::Primitive {
        nesting: 0,
        data_offset: 0,
        topology_offset: 0,
        fixed_size: 0,
        nullable_bit_index: 0,
        group: false,
        group_nest: false,
    },
}];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_str_combining() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::Str,
        }],
    };

    let data = PyroValue::Str("e\u{0301}".into());

    run_list_wrapped_roundtrip_u32(schema, data, &STR_COMBINING, &STR_COMBINING_INODES);
}

// Right-to-left script (Hebrew): the wire format stores bytes, not
// glyphs, so bidi rendering direction is a display-layer concern that
// shouldn't affect the byte-for-byte roundtrip at all — this case just
// confirms that's actually true.
pub const STR_RTL: [u8; 14] = [
    8, 0, 0, 0, // data pointer
    6, 0, 0, 0, // data length
    215, 144, 215, 145, 215,
    146, // "אבג" (Hebrew aleph-bet-gimel), 2 UTF-8 bytes per codepoint
];
pub const STR_RTL_INODES: [Inode; 1] = [Inode {
    kind: WireKind::Str,
    kind_meta: KindMeta::Primitive {
        nesting: 0,
        data_offset: 0,
        topology_offset: 0,
        fixed_size: 0,
        nullable_bit_index: 0,
        group: false,
        group_nest: false,
    },
}];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_str_rtl() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::Str,
        }],
    };

    let data = PyroValue::Str("אבג".into());

    run_list_wrapped_roundtrip_u32(schema, data, &STR_RTL, &STR_RTL_INODES);
}

// A ZWJ emoji sequence: three 4-byte emoji codepoints glued into a single
// displayed "family" glyph by two U+200D ZERO WIDTH JOINER codepoints (3
// bytes each) in between — six codepoints, one grapheme, 18 UTF-8 bytes.
// The most adversarial case here: it stresses both the outside-BMP 4-byte
// encoding (`STR_EMOJI`) and multi-codepoint-per-grapheme (`STR_COMBINING`)
// at once, at a length long enough to catch an off-by-one in the byte
// count that a shorter case might miss.
pub const STR_ZWJ_SEQUENCE: [u8; 26] = [
    8, 0, 0, 0, // data pointer
    18, 0, 0, 0, // data length
    240, 159, 145, 168, // "👨" MAN (U+1F468), 4 bytes
    226, 128, 141, // U+200D ZERO WIDTH JOINER, 3 bytes
    240, 159, 145, 169, // "👩" WOMAN (U+1F469), 4 bytes
    226, 128, 141, // U+200D ZERO WIDTH JOINER, 3 bytes
    240, 159, 145, 167, // "👧" GIRL (U+1F467), 4 bytes
];
pub const STR_ZWJ_SEQUENCE_INODES: [Inode; 1] = [Inode {
    kind: WireKind::Str,
    kind_meta: KindMeta::Primitive {
        nesting: 0,
        data_offset: 0,
        topology_offset: 0,
        fixed_size: 0,
        nullable_bit_index: 0,
        group: false,
        group_nest: false,
    },
}];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_str_zwj_sequence() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::Str,
        }],
    };

    let data = PyroValue::Str("\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}".into());

    run_list_wrapped_roundtrip_u32(schema, data, &STR_ZWJ_SEQUENCE, &STR_ZWJ_SEQUENCE_INODES);
}

pub const BYTES_BASIC: [u8; 13] = [
    8, 0, 0, 0, // data pointer
    5, 0, 0, 0, // data length
    1, 2, 3, 4, 5,
];
pub const BYTES_BASIC_INODES: [Inode; 1] = [Inode {
    kind: WireKind::Bytes,
    kind_meta: KindMeta::Primitive {
        nesting: 0,
        data_offset: 0,
        topology_offset: 0,
        fixed_size: 0,
        nullable_bit_index: 0,
        group: false,
        group_nest: false,
    },
}];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_bytes_basic() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::Bytes,
        }],
    };

    let data = PyroValue::Bytes(vec![1, 2, 3, 4, 5].into());

    run_list_wrapped_roundtrip_u32(schema, data, &BYTES_BASIC, &BYTES_BASIC_INODES);
}

pub const BYTES_EMPTY: [u8; 8] = [
    8, 0, 0, 0, // data pointer
    0, 0, 0, 0, // data length — no payload bytes follow
];
pub const BYTES_EMPTY_INODES: [Inode; 1] = [Inode {
    kind: WireKind::Bytes,
    kind_meta: KindMeta::Primitive {
        nesting: 0,
        data_offset: 0,
        topology_offset: 0,
        fixed_size: 0,
        nullable_bit_index: 0,
        group: false,
        group_nest: false,
    },
}];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_bytes_empty() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::Bytes,
        }],
    };

    let data = PyroValue::Bytes(vec![].into());

    run_list_wrapped_roundtrip_u32(schema, data, &BYTES_EMPTY, &BYTES_EMPTY_INODES);
}
