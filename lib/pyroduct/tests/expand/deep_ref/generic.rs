//! Test FromRow with nested structs

use pyroduct::DeepRef;

#[derive(DeepRef)]
struct Address {
    street: String,
    city: String,
    zip: u32,
}

#[derive(DeepRef)]
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
}