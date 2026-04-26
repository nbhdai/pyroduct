use pyroduct::format::Bridgeable;
use pyroduct::format::value::{PyroRow, PyroValue};

#[test]
fn test_bridgeable_i32() {
    let original = 42i32;
    let vec = original.ship().unwrap();
    let exposed = i32::expose(vec).unwrap();
    assert_eq!(*exposed.inner(), 42);
}

#[test]
fn test_bridgeable_string() {
    let original = "hello".to_string();
    let vec = original.ship().unwrap();
    let exposed = String::expose(vec).unwrap();
    assert_eq!(*exposed.inner(), "hello");
}

#[test]
fn test_bridgeable_vec_u8() {
    let original = vec![1u8, 2, 3];
    let vec = original.ship().unwrap();
    let exposed = <Vec<u8>>::expose(vec).unwrap();
    assert_eq!(*exposed.inner(), &[1u8, 2, 3]);
}
