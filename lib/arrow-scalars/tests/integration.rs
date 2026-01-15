//! Integration tests for arrow-derive macros
//!
//! These tests verify the runtime behavior of derived implementations

use arrow_scalars::{ArrowRow, ArrowValue, DeepRef, FromRow, ToRow};

#[test]
fn test_from_arrow_basic_primitives() {
    #[allow(dead_code)]
    #[derive(FromRow, DeepRef)]
    struct Data {
        a: i32,
        b: f64,
        c: bool,
    }

    let row = ArrowRow::from([
        ("a", ArrowValue::I32(42)),
        ("b", ArrowValue::F64(3.14)),
        ("c", ArrowValue::Bool(true)),
    ]);

    let data = DataRef::from_row(&row).unwrap();

    assert_eq!(data.a, 42);
    assert_eq!(data.b, 3.14);
    assert_eq!(data.c, true);
}

#[test]
fn test_from_arrow_string_becomes_str() {
    #[allow(dead_code)]
    #[derive(FromRow, DeepRef)]
    struct Data {
        name: String,
    }

    let row = ArrowRow::from([("name", ArrowValue::from("test"))]);
    let data = DataRef::from_row(&row).unwrap();

    // Verify it's a &str, not String
    let _: &str = data.name;
    assert_eq!(data.name, "test");
}

#[test]
fn test_from_arrow_vec_becomes_slice() {
    #[allow(dead_code)]
    #[derive(FromRow, DeepRef)]
    struct Data {
        numbers: Vec<i32>,
    }

    let row = ArrowRow::from([("numbers", ArrowValue::from(&[1, 2, 3][..]))]);
    let data = DataRef::from_row(&row).unwrap();

    // Verify it's a slice
    let _: &[i32] = data.numbers;
    assert_eq!(data.numbers, &[1, 2, 3]);
}

#[test]
fn test_from_arrow_option_some() {
    #[allow(dead_code)]
    #[derive(FromRow, DeepRef)]
    struct Data {
        maybe_num: Option<i32>,
        maybe_str: Option<String>,
    }

    let row = ArrowRow::from([
        ("maybe_num", ArrowValue::I32(100)),
        ("maybe_str", ArrowValue::from("present")),
    ]);

    let data = DataRef::from_row(&row).unwrap();

    assert_eq!(data.maybe_num, Some(100));
    assert_eq!(data.maybe_str, Some("present"));
}

#[test]
fn test_from_arrow_option_none() {
    #[allow(dead_code)]
    #[derive(FromRow, DeepRef)]
    struct Data {
        maybe_num: Option<i32>,
        maybe_str: Option<String>,
    }

    let row = ArrowRow::from([
        ("maybe_num", ArrowValue::Null),
        ("maybe_str", ArrowValue::Null),
    ]);

    let data = DataRef::from_row(&row).unwrap();

    assert_eq!(data.maybe_num, None);
    assert_eq!(data.maybe_str, None);
}

#[test]
fn test_from_arrow_nested_struct() {
    #[allow(dead_code)]
    #[derive(FromRow, DeepRef)]
    struct Inner {
        value: i32,
    }

    #[allow(dead_code)]
    #[derive(FromRow, DeepRef)]
    struct Outer {
        name: String,
        inner: Inner,
    }

    let inner_row = ArrowRow::from([("value", ArrowValue::I32(99))]);
    let outer_row = ArrowRow::from([
        ("name", ArrowValue::from("test")),
        ("inner", ArrowValue::Group(inner_row)),
    ]);

    let data = OuterRef::from_row(&outer_row).unwrap();

    assert_eq!(data.name, "test");
    assert_eq!(data.inner.value, 99);
}

#[test]
fn test_from_arrow_missing_field_error() {
    #[allow(dead_code)]
    #[derive(FromRow, DeepRef)]
    struct Data {
        required: i32,
    }

    let row = ArrowRow::new();
    let result = DataRef::from_row(&row);
    println!("{result:?}");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Missing Field: required"));
}

#[test]
fn test_from_arrow_wrong_type_error() {
    #[allow(dead_code)]
    #[derive(FromRow, DeepRef)]
    struct Data {
        num: i32,
    }

    // Pass a string instead of i32
    let row = ArrowRow::from([("num", ArrowValue::from("not a number"))]);
    let result = DataRef::from_row(&row);

    assert!(result.is_err());
}

#[test]
fn test_to_arrow_basic() {
    #[derive(ToRow)]
    struct Data {
        id: u32,
        name: String,
        active: bool,
    }

    let data = Data {
        id: 42,
        name: "test".to_string(),
        active: true,
    };

    let row = data.to_row();

    assert_eq!(row.get("id"), Some(&ArrowValue::U32(42)));
    assert_eq!(row.get("name"), Some(&ArrowValue::from("test")));
    assert_eq!(row.get("active"), Some(&ArrowValue::Bool(true)));
}

#[test]
fn test_to_arrow_option_some() {
    #[derive(ToRow)]
    struct Data {
        maybe: Option<i32>,
    }

    let data = Data { maybe: Some(42) };
    let row = data.to_row();

    assert_eq!(row.get("maybe"), Some(&ArrowValue::I32(42)));
}

#[test]
fn test_to_arrow_option_none() {
    #[derive(ToRow)]
    struct Data {
        maybe: Option<i32>,
    }

    let data = Data { maybe: None };
    let row = data.to_row();

    assert_eq!(row.get("maybe"), Some(&ArrowValue::Null));
}

#[test]
fn test_to_arrow_nested() {
    #[derive(ToRow)]
    struct Inner {
        value: i32,
    }

    #[derive(ToRow)]
    struct Outer {
        inner: Inner,
    }

    let data = Outer {
        inner: Inner { value: 99 },
    };

    let row = data.to_row();

    if let Some(ArrowValue::Group(inner_row)) = row.get("inner") {
        assert_eq!(inner_row.get("value"), Some(&ArrowValue::I32(99)));
    } else {
        panic!("Expected nested Group");
    }
}

#[test]
fn test_deep_ref_primitives() {
    #[derive(FromRow, DeepRef)]
    struct Data {
        id: u32,
        name: String,
    }

    let data = Data {
        id: 42,
        name: "test".to_string(),
    };

    let data_ref = data.as_deep_ref();

    assert_eq!(data_ref.id, 42);
    assert_eq!(data_ref.name, "test");

    // Verify string is borrowed
    let _: &str = data_ref.name;
}

#[test]
fn test_deep_ref_vec_primitives() {
    #[derive(FromRow, DeepRef)]
    struct Data {
        scores: Vec<i32>,
    }

    let data = Data {
        scores: vec![10, 20, 30],
    };

    let data_ref = data.as_deep_ref();

    assert_eq!(data_ref.scores, &[10, 20, 30]);
    let _: &[i32] = data_ref.scores;
}

#[test]
fn test_deep_ref_option() {
    #[derive(FromRow, DeepRef)]
    struct Data {
        maybe_str: Option<String>,
        maybe_num: Option<i32>,
    }

    let data1 = Data {
        maybe_str: Some("test".to_string()),
        maybe_num: Some(42),
    };

    let ref1 = data1.as_deep_ref();
    assert_eq!(ref1.maybe_str, Some("test"));
    assert_eq!(ref1.maybe_num, Some(42));

    let data2 = Data {
        maybe_str: None,
        maybe_num: None,
    };

    let ref2 = data2.as_deep_ref();
    assert_eq!(ref2.maybe_str, None);
    assert_eq!(ref2.maybe_num, None);
}

#[test]
fn test_roundtrip_conversion() {
    #[derive(FromRow, ToRow, DeepRef)]
    struct Data {
        id: u32,
        name: String,
        scores: Vec<i32>,
    }

    let original = Data {
        id: 42,
        name: "alice".to_string(),
        scores: vec![1, 2, 3],
    };

    // Convert to ArrowValue
    let arrow_val = original.to_row();

    // Parse back
    let parsed = DataRef::from_row(&arrow_val).unwrap();

    // Compare via DeepRef
    let original_ref = original.as_deep_ref();
    assert_eq!(parsed.id, original_ref.id);
    assert_eq!(parsed.name, original_ref.name);
    assert_eq!(parsed.scores, original_ref.scores);
}

#[test]
fn test_all_numeric_types() {
    #[derive(FromRow, ToRow, DeepRef)]
    struct AllNumbers {
        i8_val: i8,
        i16_val: i16,
        i32_val: i32,
        i64_val: i64,
        u8_val: u8,
        u16_val: u16,
        u32_val: u32,
        u64_val: u64,
        f32_val: f32,
        f64_val: f64,
    }

    let data = AllNumbers {
        i8_val: -1,
        i16_val: -100,
        i32_val: -1000,
        i64_val: -10000,
        u8_val: 1,
        u16_val: 100,
        u32_val: 1000,
        u64_val: 10000,
        f32_val: 3.14,
        f64_val: 2.718,
    };

    // Test ToRow
    let row = data.to_row();
    assert_eq!(row.get("i8_val"), Some(&ArrowValue::I8(-1)));
    assert_eq!(row.get("u32_val"), Some(&ArrowValue::U32(1000)));
    assert_eq!(row.get("f64_val"), Some(&ArrowValue::F64(2.718)));

    // Test round-trip
    let arrow_val = data.to_row();
    let parsed = AllNumbersRef::from_row(&arrow_val).unwrap();
    assert_eq!(parsed.i32_val, -1000);
    assert_eq!(parsed.u64_val, 10000);
}

#[test]
fn test_complex_nested_structure() {
    #[derive(FromRow, ToRow, DeepRef)]
    struct Address {
        street: String,
        zip: u32,
    }

    #[derive(FromRow, ToRow, DeepRef)]
    struct Person {
        name: String,
        age: i32,
        address: Address,
        tags: Vec<String>,
    }

    let person = Person {
        name: "Alice".to_string(),
        age: 30,
        address: Address {
            street: "Main St".to_string(),
            zip: 12345,
        },
        tags: vec!["tag1".to_string(), "tag2".to_string()],
    };

    // Convert to Arrow and back
    let arrow_val = person.to_row();
    let parsed = PersonRef::from_row(&arrow_val).unwrap();

    assert_eq!(parsed.name, "Alice");
    assert_eq!(parsed.age, 30);
    assert_eq!(parsed.address.street, "Main St");
    assert_eq!(parsed.address.zip, 12345);
}

#[test]
fn test_empty_struct() {
    #[derive(FromRow, ToRow, DeepRef)]
    struct Empty {}

    let data = Empty {};
    let row = data.to_row();
    assert_eq!(row.len(), 0);

    let arrow_val = data.to_row();
    let _parsed = EmptyRef::from_row(&arrow_val).unwrap();
}

#[test]
fn test_single_field_struct() {
    #[derive(FromRow, ToRow, DeepRef)]
    struct Single {
        value: i32,
    }

    let data = Single { value: 42 };
    let row = data.to_row();
    assert_eq!(row.get("value"), Some(&ArrowValue::I32(42)));

    let arrow_val = data.to_row();
    let parsed = SingleRef::from_row(&arrow_val).unwrap();
    assert_eq!(parsed.value, 42);
}

#[test]
fn test_vec_of_all_primitive_types() {
    #[derive(FromRow, ToRow, DeepRef)]
    struct VecData {
        i32_vec: Vec<i32>,
        u64_vec: Vec<u64>,
        f64_vec: Vec<f64>,
        bool_vec: Vec<bool>,
    }

    let data = VecData {
        i32_vec: vec![1, 2, 3],
        u64_vec: vec![100, 200],
        f64_vec: vec![1.1, 2.2],
        bool_vec: vec![true, false, true],
    };

    let arrow_val = data.to_row();
    let parsed = VecDataRef::from_row(&arrow_val).unwrap();

    assert_eq!(parsed.i32_vec, &[1, 2, 3]);
    assert_eq!(parsed.u64_vec, &[100, 200]);
    assert_eq!(parsed.f64_vec, &[1.1, 2.2]);
    assert_eq!(parsed.bool_vec, &[true, false, true]);
}
