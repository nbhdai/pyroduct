//! Test FromRow with nested structs

use pyroduct::{FromRow, RefFromRow, DeepRef, PyroValue, PyroRow};

#[derive(FromRow, RefFromRow, DeepRef)]
struct Address {
    street: String,
    city: String,
    zip: u32,
}

#[derive(FromRow, RefFromRow, DeepRef)]
struct Person {
    name: String,
    age: i32,
    address: Address,
}

fn main() {
    let addr_row = PyroRow::from([
        ("street", PyroValue::from("123 Main St")),
        ("city", PyroValue::from("Springfield")),
        ("zip", PyroValue::U32(12345)),
    ]);

    let person_row = PyroRow::from([
        ("name", PyroValue::from("Bob")),
        ("age", PyroValue::I32(30)),
        ("address", PyroValue::Group(addr_row)),
    ]);

    let p = PersonRef::try_from(&person_row).unwrap();
    
    assert_eq!(p.name, "Bob");
    assert_eq!(p.age, 30);
    assert_eq!(p.address.street, "123 Main St");
    assert_eq!(p.address.city, "Springfield");
    assert_eq!(p.address.zip, 12345);
}