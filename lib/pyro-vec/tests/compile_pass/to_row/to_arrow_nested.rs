//! Test ToRow with nested structs

use pyro_vec::{ToRow, ArrowValue};

#[derive(ToRow)]
struct Address {
    street: String,
    zip: u32,
}

#[derive(ToRow)]
struct Person {
    name: String,
    address: Address,
}

fn main() {
    let person = Person {
        name: "Alice".to_string(),
        address: Address {
            street: "123 Main".to_string(),
            zip: 12345,
        },
    };
    
    let row = person.to_row();
    
    assert_eq!(row.get("name"), Some(&ArrowValue::from("Alice")));
    
    // Nested struct should be ArrowValue::Group
    if let Some(ArrowValue::Group(addr_row)) = row.get("address") {
        assert_eq!(addr_row.get("street"), Some(&ArrowValue::from("123 Main")));
        assert_eq!(addr_row.get("zip"), Some(&ArrowValue::U32(12345)));
    } else {
        panic!("Expected nested Group for address");
    }
}