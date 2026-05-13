// //! Integration tests for Bridgeable derive macro

use pyroduct::{
    format::{Bridgeable, bridgeable::BridgeableZeroCopy, format::Receiver},
    magma,
};

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
fn test_simple_struct_matches_rkyv() {
    let val = SimpleStruct {
        id: 42,
        name: "Hello Rkyv".to_string(),
    };

    // Serialize via Bridgable
    let vec = val.ship().expect("Bridgable serialization failed");

    // Serialize via rkyv directly
    let correct_vec =
        rkyv::to_bytes::<rkyv::rancor::Error>(&val).expect("rkyv serialization failed");

    // Check alignment
    let data_addr = vec.data_ptr() as usize;
    assert_eq!(
        data_addr % 16,
        0,
        "Data payload must start on 16-byte boundary"
    );

    // Debug output
    println!("PyroVec: {:?}", vec);
    println!("PyroVec bytes: {:x?}", vec.as_slice());
    println!("rkyv bytes len: {}", correct_vec.len());
    println!("rkyv bytes: {:x?}", correct_vec.as_slice());

    // Verify bytes match
    assert_eq!(
        vec.as_slice(),
        correct_vec.as_slice(),
        "Serialized bytes must match rkyv output"
    );

    // Verify we can access the archived data
    let slice = vec.as_slice();
    let access_result = rkyv::access::<ArchivedSimpleStruct, rkyv::rancor::Error>(slice);

    if let Err(e) = &access_result {
        panic!("Access failed! Error: {:?}", e);
    }

    let archived = access_result.unwrap();
    assert_eq!(archived.id, 42);
    assert_eq!(archived.name.as_str(), "Hello Rkyv");
}

#[test]
fn test_with_vec_matches_rkyv() {
    let val = WithVec {
        items: vec![1, 2, 3, 4, 5],
        labels: vec!["a".to_string(), "b".to_string(), "c".to_string()],
    };

    let vec = val.ship().expect("Bridgable serialization failed");
    let correct_vec =
        rkyv::to_bytes::<rkyv::rancor::Error>(&val).expect("rkyv serialization failed");

    assert_eq!(
        vec.as_slice(),
        correct_vec.as_slice(),
        "Serialized bytes must match rkyv output"
    );

    let slice = vec.as_slice();
    let access_result = rkyv::access::<ArchivedWithVec, rkyv::rancor::Error>(slice);

    if let Err(e) = &access_result {
        panic!("Access failed! Error: {:?}", e);
    }

    let archived = access_result.unwrap();
    assert_eq!(archived.items.len(), 5);
    assert_eq!(archived.labels.len(), 3);
}

#[test]
fn test_nested_matches_rkyv() {
    let val = Nested {
        inner: SimpleStruct {
            id: 100,
            name: "nested".to_string(),
        },
        count: 999,
    };

    let vec = val.ship().expect("Bridgable serialization failed");
    let correct_vec =
        rkyv::to_bytes::<rkyv::rancor::Error>(&val).expect("rkyv serialization failed");

    assert_eq!(
        vec.as_slice(),
        correct_vec.as_slice(),
        "Serialized bytes must match rkyv output"
    );

    let slice = vec.as_slice();
    let access_result = rkyv::access::<ArchivedNested, rkyv::rancor::Error>(slice);

    if let Err(e) = &access_result {
        panic!("Access failed! Error: {:?}", e);
    }

    let archived = access_result.unwrap();
    assert_eq!(archived.inner.id, 100);
    assert_eq!(archived.count, 999);
}

#[test]
fn test_roundtrip_via_bridgable() {
    let original = SimpleStruct {
        id: 12345,
        name: "roundtrip".to_string(),
    };

    let vec = original.ship().expect("serialize failed");
    let typed = SimpleStruct::expose(vec.view()).expect("parse failed");
    let mut receiver = SimpleStruct::receiver();
    let recovered = receiver.receive(&typed).expect("deserialize failed");

    assert_eq!(original, recovered);
}

#[test]
fn test_large_data_matches_rkyv() {
    let val = WithVec {
        items: (0..10_000).map(|i| (i % 256) as u8).collect(),
        labels: (0..1_000).map(|i| format!("label_{}", i)).collect(),
    };

    let vec = val.ship().expect("Bridgable serialization failed");
    let correct_vec =
        rkyv::to_bytes::<rkyv::rancor::Error>(&val).expect("rkyv serialization failed");

    assert_eq!(
        vec.as_slice(),
        correct_vec.as_slice(),
        "Serialized bytes must match rkyv output"
    );
}

#[test]
fn test_empty_collections_matches_rkyv() {
    let val = WithVec {
        items: Vec::new(),
        labels: Vec::new(),
    };

    let vec = val.ship().expect("Bridgable serialization failed");
    let correct_vec =
        rkyv::to_bytes::<rkyv::rancor::Error>(&val).expect("rkyv serialization failed");

    assert_eq!(
        vec.as_slice(),
        correct_vec.as_slice(),
        "Serialized bytes must match rkyv output"
    );
}

#[test]
fn test_boundary_values_match_rkyv() {
    let val = SimpleStruct {
        id: u32::MAX,
        name: String::new(),
    };

    let vec = val.ship().expect("Bridgable serialization failed");
    let correct_vec =
        rkyv::to_bytes::<rkyv::rancor::Error>(&val).expect("rkyv serialization failed");

    assert_eq!(
        vec.as_slice(),
        correct_vec.as_slice(),
        "Serialized bytes must match rkyv output"
    );
}
