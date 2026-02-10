//! Test FromRow with nested structs

use pyro_vec::{FromRow, DeepRef, ArrowValue, ArrowRow};

#[derive(FromRow, DeepRef)]
struct Address {
    street: String,
    city: String,
    zip: u32,
}

#[derive(FromRow, DeepRef)]
struct Person {
    name: String,
    age: i32,
    address: Address,
}

fn main() {
    let addr_row = ArrowRow::from([
        ("street", ArrowValue::from("123 Main St")),
        ("city", ArrowValue::from("Springfield")),
        ("zip", ArrowValue::U32(12345)),
    ]);

    let person_row = ArrowRow::from([
        ("name", ArrowValue::from("Bob")),
        ("age", ArrowValue::I32(30)),
        ("address", ArrowValue::Group(addr_row)),
    ]);

    let p = PersonRef::from_row(&person_row).unwrap();
    
    assert_eq!(p.name, "Bob");
    assert_eq!(p.age, 30);
    assert_eq!(p.address.street, "123 Main St");
    assert_eq!(p.address.city, "Springfield");
    assert_eq!(p.address.zip, 12345);
}