//! Test round-trip: Owned -> ArrowValue -> Ref -> compare

use pyroduct::format::{DeepRef, FromRow, ToRow};

#[derive(FromRow, DeepRef, ToRow)]
struct ComplexData {
    id: u32,
    name: String,
    scores: Vec<i32>,
    metadata: Option<String>,
}

fn main() {
    let original = ComplexData {
        id: 42,
        name: "test".to_string(),
        scores: vec![10, 20, 30],
        metadata: Some("extra".to_string()),
    };

    // Convert to ArrowValue
    let arrow_value = original.to_row();

    // Parse back to Ref
    let parsed = ComplexDataRef::from_row(&arrow_value).unwrap();

    // Compare via DeepRef
    let original_ref = original.as_deep_ref();

    assert_eq!(parsed.id, original_ref.id);
    assert_eq!(parsed.name, original_ref.name);
    assert_eq!(parsed.scores, original_ref.scores);
    assert_eq!(parsed.metadata, original_ref.metadata);
}
