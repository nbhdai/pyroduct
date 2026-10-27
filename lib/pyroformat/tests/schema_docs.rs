//! Documentation flows end to end: `///` comments on a `#[derive(Layout)]`
//! struct are baked into its static tables, realized into a `PyroSchema`,
//! encoded into the wire metadata string table, and recovered on decode.

use pyroformat::Layout;
use pyroformat::schema::{PyroSchema, SchemaParts};

/// `T`'s schema, realized from its `Layout` tables.
fn schema_of<T: Layout>() -> PyroSchema<'static> {
    SchemaParts {
        inodes: T::INODES.to_vec(),
        field: T::FLAT_NAMES.to_vec(),
        documentation: T::FLAT_DOCS.to_vec(),
        schema_documentation: Some(T::DOC),
        meta: T::META,
    }
    .to_schema()
    .expect("a derived layout realizes")
}

/// A street address.
#[derive(Layout, Debug, PartialEq)]
struct Address {
    /// Street line, house number included.
    street: String,
    /// Postal code.
    zip: u32,
    // Deliberately undocumented.
    country: String,
}

/// A person on file.
///
/// Second line of the struct doc.
#[derive(Layout, Debug, PartialEq)]
struct Person {
    /// Unique identifier.
    id: u64,
    /// Where they live.
    home: Address,
    age: i32,
}

#[test]
fn struct_and_field_docs_are_baked_into_statics() {
    assert_eq!(Address::DOC, "A street address.");
    assert_eq!(
        Person::DOC,
        "A person on file.\n\nSecond line of the struct doc."
    );

    let schema = schema_of::<Person>();
    let names: Vec<&str> = schema.fields().map(|f| f.name()).collect();
    assert_eq!(names, ["id", "home", "age"]);
    let docs: Vec<&str> = schema.fields().map(|f| f.doc()).collect();
    assert_eq!(docs, ["Unique identifier.", "Where they live.", ""]);
}

#[test]
fn flat_tables_descend_into_nested_structs() {
    assert_eq!(
        Person::FLAT_NAMES,
        &["id", "home", "street", "zip", "country", "age"]
    );
    assert_eq!(
        Person::FLAT_DOCS,
        &[
            "Unique identifier.",
            "Where they live.",
            "Street line, house number included.",
            "Postal code.",
            "",
            "",
        ]
    );
}

#[test]
fn realized_schema_carries_documentation() {
    let schema = schema_of::<Person>();

    assert_eq!(schema.doc(), Person::DOC);
    assert_eq!(schema.num_fields(), 3);

    let home = schema.field(1);
    assert_eq!(home.name(), "home");
    assert_eq!(home.doc(), "Where they live.");

    let mut children = home.group_children().expect("home is a Group");
    let street = children.next().unwrap();
    assert_eq!(street.name(), "street");
    assert_eq!(street.doc(), "Street line, house number included.");
    let zip = children.next().unwrap();
    assert_eq!(zip.doc(), "Postal code.");
    let country = children.next().unwrap();
    assert_eq!(country.doc(), "");
}

#[test]
fn documentation_survives_a_wire_roundtrip() {
    let schema = schema_of::<Person>();

    let with_docs = schema.encode_with_doc(true).expect("encode");
    let decoded = PyroSchema::decode(&with_docs).expect("decode");

    assert_eq!(decoded.doc(), Person::DOC);
    assert_eq!(decoded.names(), schema.names());
    assert_eq!(decoded.field_docs(), schema.field_docs());
    assert_eq!(decoded.types(), schema.types());

    let home = decoded.field(1);
    assert_eq!(home.doc(), "Where they live.");
    let street = home.group_children().unwrap().next().unwrap();
    assert_eq!(street.doc(), "Street line, house number included.");

    // Opting out drops the docs but keeps names and types intact.
    let without = schema.encode_with_doc(false).expect("encode");
    let decoded = PyroSchema::decode(&without).expect("decode");
    assert_eq!(decoded.doc(), "");
    assert_eq!(decoded.names(), schema.names());
    assert_eq!(decoded.types(), schema.types());
    assert!(decoded.field_docs().iter().all(|d| d.is_empty()));
}
