//! Test FromRow with nested structs

use pyroduct::{DeepRef, DeepRefArchived, rkyv_8::rkyv::{Archive, Serialize, Deserialize}};

#[derive(DeepRef, DeepRefArchived, Archive, Serialize, Deserialize)]
struct Address {
    street: String,
    city: String,
    zip: u32,
}

#[derive(DeepRef, DeepRefArchived, Archive, Serialize, Deserialize)]
struct Person<T> {
    name: String,
    age: i32,
    address: T,
}

fn main() {
    let person = Person::<Address> {
        name: "Bob".to_string(),
        age: 30,
        address: Address {
            street: "123 Main St".to_string(),
            city: "Springfield".to_string(),
            zip: 12345,
        }
    };

    let p_ref = person.as_deep_ref();
    
    assert_eq!(p_ref.name, "Bob");
    assert_eq!(p_ref.age, 30);
    assert_eq!(p_ref.address.street, "123 Main St");
    assert_eq!(p_ref.address.city, "Springfield");
    assert_eq!(p_ref.address.zip, 12345);
}