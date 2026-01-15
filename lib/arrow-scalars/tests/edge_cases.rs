//! Edge case tests for arrow-derive macros

use arrow_scalars::{ArrowRow, ArrowValue, DeepRef, FromRow, ToRow};

#[test]
fn test_deeply_nested_structures() {
    #[derive(FromRow, DeepRef, ToRow)]
    struct Level3 {
        value: i32,
    }

    #[derive(FromRow, DeepRef, ToRow)]
    struct Level2 {
        inner: Level3,
    }

    #[derive(FromRow, DeepRef, ToRow)]
    struct Level1 {
        inner: Level2,
    }

    let data = Level1 {
        inner: Level2 {
            inner: Level3 { value: 42 },
        },
    };

    let arrow_val = data.to_row();
    let parsed = Level1Ref::from_row(&arrow_val).unwrap();

    assert_eq!(parsed.inner.inner.value, 42);
}

#[test]
fn test_option_of_nested_struct() {
    #[derive(FromRow, DeepRef, ToRow)]
    struct Inner {
        value: i32,
    }

    #[derive(FromRow, DeepRef, ToRow)]
    struct Outer {
        maybe_inner: Option<Inner>,
    }

    // Test with Some
    let data1 = Outer {
        maybe_inner: Some(Inner { value: 99 }),
    };

    let arrow_val1 = data1.to_row();
    let parsed1 = OuterRef::from_row(&arrow_val1).unwrap();

    if let Some(inner) = parsed1.maybe_inner {
        assert_eq!(inner.value, 99);
    } else {
        panic!("Expected Some");
    }

    // Test with None
    let data2 = Outer { maybe_inner: None };

    let arrow_val2 = data2.to_row();
    let parsed2 = OuterRef::from_row(&arrow_val2).unwrap();

    assert!(parsed2.maybe_inner.is_none());
}

#[test]
fn test_option_of_vec() {
    #[derive(FromRow, DeepRef, ToRow)]
    struct Data {
        maybe_list: Option<Vec<i32>>,
    }

    // Test with Some
    let data1 = Data {
        maybe_list: Some(vec![1, 2, 3]),
    };

    let arrow_val1 = data1.to_row();
    let parsed1 = DataRef::from_row(&arrow_val1).unwrap();

    if let Some(list) = parsed1.maybe_list {
        assert_eq!(list, &[1, 2, 3]);
    } else {
        panic!("Expected Some");
    }

    // Test with None
    let data2 = Data { maybe_list: None };

    let arrow_val2 = data2.to_row();
    let parsed2 = DataRef::from_row(&arrow_val2).unwrap();

    assert!(parsed2.maybe_list.is_none());
}

#[test]
fn test_empty_vec() {
    #[derive(FromRow, DeepRef, ToRow)]
    struct Data {
        empty_list: Vec<i32>,
    }

    let data = Data { empty_list: vec![] };

    let arrow_val = data.to_row();
    let parsed = DataRef::from_row(&arrow_val).unwrap();
    let empty_list: &[i32] = &[];
    assert_eq!(parsed.empty_list, empty_list);
}

#[test]
fn test_empty_string() {
    #[derive(FromRow, DeepRef, ToRow)]
    struct Data {
        text: String,
    }

    let data = Data {
        text: String::new(),
    };

    let arrow_val = data.to_row();
    let parsed = DataRef::from_row(&arrow_val).unwrap();

    assert_eq!(parsed.text, "");
}

#[test]
fn test_large_vec() {
    #[derive(FromRow, DeepRef, ToRow)]
    struct Data {
        numbers: Vec<i32>,
    }

    let large_vec: Vec<i32> = (0..10000).collect();
    let data = Data {
        numbers: large_vec.clone(),
    };

    let arrow_val = data.to_row();
    let parsed = DataRef::from_row(&arrow_val).unwrap();

    assert_eq!(parsed.numbers.len(), 10000);
    assert_eq!(parsed.numbers[0], 0);
    assert_eq!(parsed.numbers[9999], 9999);
}

#[test]
fn test_unicode_strings() {
    #[derive(FromRow, DeepRef, ToRow)]
    struct Data {
        text: String,
    }

    let data = Data {
        text: "Hello 世界 🌍".to_string(),
    };

    let arrow_val = data.to_row();
    let parsed = DataRef::from_row(&arrow_val).unwrap();

    assert_eq!(parsed.text, "Hello 世界 🌍");
}

#[test]
fn test_all_none_options() {
    #[derive(FromRow, DeepRef, ToRow)]
    struct Data {
        opt1: Option<i32>,
        opt2: Option<String>,
        opt3: Option<Vec<i32>>,
    }

    let data = Data {
        opt1: None,
        opt2: None,
        opt3: None,
    };

    let arrow_val = data.to_row();
    let parsed = DataRef::from_row(&arrow_val).unwrap();

    assert!(parsed.opt1.is_none());
    assert!(parsed.opt2.is_none());
    assert!(parsed.opt3.is_none());
}

#[test]
fn test_all_some_options() {
    #[derive(FromRow, DeepRef, ToRow)]
    struct Data {
        opt1: Option<i32>,
        opt2: Option<String>,
        opt3: Option<Vec<i32>>,
    }

    let data = Data {
        opt1: Some(42),
        opt2: Some("test".to_string()),
        opt3: Some(vec![1, 2, 3]),
    };

    let arrow_val = data.to_row();
    let parsed = DataRef::from_row(&arrow_val).unwrap();

    assert_eq!(parsed.opt1, Some(42));
    assert_eq!(parsed.opt2, Some("test"));
    assert_eq!(parsed.opt3.map(|v| v.to_vec()), Some(vec![1, 2, 3]));
}

#[test]
fn test_multiple_nesting_levels_with_options() {
    #[derive(FromRow, DeepRef, ToRow)]
    struct DeepInner {
        value: i32,
    }

    #[derive(FromRow, DeepRef, ToRow)]
    struct Middle {
        maybe_inner: Option<DeepInner>,
    }

    #[derive(FromRow, DeepRef, ToRow)]
    struct Outer {
        maybe_middle: Option<Middle>,
    }

    let data = Outer {
        maybe_middle: Some(Middle {
            maybe_inner: Some(DeepInner { value: 123 }),
        }),
    };

    let arrow_val = data.to_row();
    let parsed = OuterRef::from_row(&arrow_val).unwrap();

    if let Some(middle) = parsed.maybe_middle {
        if let Some(inner) = middle.maybe_inner {
            assert_eq!(inner.value, 123);
        } else {
            panic!("Expected inner Some");
        }
    } else {
        panic!("Expected middle Some");
    }
}

#[test]
fn test_struct_with_all_field_types() {
    #[derive(FromRow, DeepRef, ToRow)]
    struct Comprehensive {
        // Primitives
        int: i32,
        float: f64,
        boolean: bool,

        // String
        text: String,

        // Vec
        numbers: Vec<i32>,

        // Options
        maybe_int: Option<i32>,
        maybe_text: Option<String>,
        maybe_list: Option<Vec<i32>>,
    }

    let data = Comprehensive {
        int: 42,
        float: 3.14,
        boolean: true,
        text: "test".to_string(),
        numbers: vec![1, 2, 3],
        maybe_int: Some(100),
        maybe_text: None,
        maybe_list: Some(vec![4, 5]),
    };

    let arrow_val = data.to_row();
    let parsed = ComprehensiveRef::from_row(&arrow_val).unwrap();

    assert_eq!(parsed.int, 42);
    assert_eq!(parsed.float, 3.14);
    assert_eq!(parsed.boolean, true);
    assert_eq!(parsed.text, "test");
    assert_eq!(parsed.numbers, &[1, 2, 3]);
    assert_eq!(parsed.maybe_int, Some(100));
    assert_eq!(parsed.maybe_text, None);
}

#[test]
fn test_special_field_names() {
    #[derive(FromRow, DeepRef, ToRow)]
    struct SpecialNames {
        r#type: i32,      // Reserved keyword
        _underscore: i32, // Leading underscore
        #[allow(non_snake_case)]
        camelCase: i32, // Mixed case
    }

    let data = SpecialNames {
        r#type: 1,
        _underscore: 2,
        camelCase: 3,
    };

    let arrow_val = data.to_row();
    let parsed = SpecialNamesRef::from_row(&arrow_val).unwrap();

    assert_eq!(parsed.r#type, 1);
    assert_eq!(parsed._underscore, 2);
    assert_eq!(parsed.camelCase, 3);
}

#[test]
fn test_from_arrow_extra_fields_ignored() {
    #[allow(dead_code)]
    #[derive(FromRow, DeepRef)]
    struct Data {
        required: i32,
    }

    // ArrowRow has extra fields that aren't in the struct
    let row = ArrowRow::from([
        ("required", ArrowValue::I32(42)),
        ("extra1", ArrowValue::I32(100)),
        ("extra2", ArrowValue::from("ignored")),
    ]);

    let data = DataRef::from_row(&row).unwrap();

    // Should successfully parse, ignoring extra fields
    assert_eq!(data.required, 42);
}

#[test]
fn test_numeric_edge_values() {
    #[derive(FromRow, DeepRef, ToRow)]
    struct NumericEdges {
        i8_min: i8,
        i8_max: i8,
        u8_max: u8,
        i32_min: i32,
        i32_max: i32,
        u32_max: u32,
    }

    let data = NumericEdges {
        i8_min: i8::MIN,
        i8_max: i8::MAX,
        u8_max: u8::MAX,
        i32_min: i32::MIN,
        i32_max: i32::MAX,
        u32_max: u32::MAX,
    };

    let arrow_val = data.to_row();
    let parsed = NumericEdgesRef::from_row(&arrow_val).unwrap();

    assert_eq!(parsed.i8_min, i8::MIN);
    assert_eq!(parsed.i8_max, i8::MAX);
    assert_eq!(parsed.u8_max, u8::MAX);
    assert_eq!(parsed.i32_min, i32::MIN);
    assert_eq!(parsed.i32_max, i32::MAX);
    assert_eq!(parsed.u32_max, u32::MAX);
}

#[test]
fn test_floating_point_special_values() {
    #[derive(FromRow, DeepRef, ToRow)]
    struct FloatSpecial {
        zero: f64,
        neg_zero: f64,
        infinity: f64,
        neg_infinity: f64,
        nan: f64,
    }

    let data = FloatSpecial {
        zero: 0.0,
        neg_zero: -0.0,
        infinity: f64::INFINITY,
        neg_infinity: f64::NEG_INFINITY,
        nan: f64::NAN,
    };

    let arrow_val = data.to_row();
    let parsed = FloatSpecialRef::from_row(&arrow_val).unwrap();

    assert_eq!(parsed.zero, 0.0);
    assert_eq!(parsed.neg_zero, -0.0);
    assert_eq!(parsed.infinity, f64::INFINITY);
    assert_eq!(parsed.neg_infinity, f64::NEG_INFINITY);
    assert!(parsed.nan.is_nan());
}
