//! Test DeepRef with Vec fields

use pyroduct::{DeepRef};

#[derive(DeepRef)]
struct WithVec {
    scores: Vec<i32>,
    tags: Vec<String>,
}

fn main() {
    let data = WithVec {
        scores: vec![10, 20, 30],
        tags: vec!["tag1".to_string(), "tag2".to_string()],
    };
    
    let data_ref = data.as_deep_ref();
    
    // Primitive vec becomes &[T]
    assert_eq!(data_ref.scores, &[10, 20, 30]);
    let _: &[i32] = data_ref.scores;
    
    // Vec<String> is problematic - verify it compiles
    // (current implementation returns empty slice)
    let _tags: Vec<&str> = data_ref.tags;
}