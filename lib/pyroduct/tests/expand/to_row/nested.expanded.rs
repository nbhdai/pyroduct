//! Test ToRow with nested structs
use pyroduct::format::ToRow;
struct Address {
    street: String,
    zip: u32,
}
impl ::pyroduct::format::ToRow for Address {
    fn to_row(&self) -> ::pyroduct::PyroRow<'_> {
        ::pyroduct::PyroRow::from([
            ("street", ::pyroduct::PyroValue::from(&self.street)),
            ("zip", ::pyroduct::PyroValue::from(&self.zip)),
        ])
    }
}
struct Person {
    name: String,
    address: Address,
}
impl ::pyroduct::format::ToRow for Person {
    fn to_row(&self) -> ::pyroduct::PyroRow<'_> {
        ::pyroduct::PyroRow::from([
            ("name", ::pyroduct::PyroValue::from(&self.name)),
            ("address", ::pyroduct::PyroValue::from(&self.address)),
        ])
    }
}
fn main() {
    let person = Person {
        name: "Alice".to_string(),
        address: Address {
            street: "123 Main".to_string(),
            zip: 12345,
        },
    };
    let _row = person.to_row();
}
