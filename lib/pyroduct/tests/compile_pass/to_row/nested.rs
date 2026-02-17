//! Test ToRow with nested structs

use pyroduct::ToRow;

#[derive(ToRow)]
struct Address {
    street: String,
    zip: u32,
}

#[derive(ToRow)]
struct Person {
    name: String,
    address: Vec<Address>,
}

fn main() {
    let person = Person {
        name: "Alice".to_string(),
        address: vec![Address {
            street: "123 Main".to_string(),
            zip: 12345,
        }],
    };
    
    let _row = person.to_row();
}