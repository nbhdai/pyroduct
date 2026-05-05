use pyroduct::{format::Bridgeable, magma};

#[magma]
#[derive(Debug, PartialEq)]
struct SimpleStruct {
    id: u32,
    name: String,
}

#[magma]
#[derive(Debug, PartialEq)]
struct WithVec {
    items: Vec<u8>,
    labels: Vec<String>,
}

#[magma]
#[derive(Debug, PartialEq)]
struct Nested {
    inner: SimpleStruct,
    count: u64,
}

#[test]
fn test_roundtrip_simple() {
    let original = SimpleStruct {
        id: 12345,
        name: "roundtrip".to_string(),
    };

    let vec = original.ship().expect("serialize failed");
    let typed = SimpleStruct::expose(vec).expect("parse failed");
    let recovered = typed.into_owned();

    assert_eq!(original, recovered);
}

#[test]
fn test_roundtrip_with_vec() {
    let original = WithVec {
        items: vec![1, 2, 3, 4, 5],
        labels: vec!["a".to_string(), "b".to_string(), "c".to_string()],
    };

    let vec = original.ship().expect("serialize failed");
    let typed = WithVec::expose(vec).expect("parse failed");
    let recovered = typed.into_owned();

    assert_eq!(original, recovered);
}

#[test]
fn test_roundtrip_nested() {
    let original = Nested {
        inner: SimpleStruct {
            id: 100,
            name: "nested".to_string(),
        },
        count: 999,
    };

    let vec = original.ship().expect("serialize failed");
    let typed = Nested::expose(vec).expect("parse failed");
    let recovered = typed.into_owned();

    assert_eq!(original, recovered);
}
