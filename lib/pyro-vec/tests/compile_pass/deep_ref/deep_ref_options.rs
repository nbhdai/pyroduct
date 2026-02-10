//! Test DeepRef with Option fields

use pyro_vec::{DeepRef};

#[derive(DeepRef)]
struct WithOption {
    id: u32,
    name: Option<String>,
    count: Option<i32>,
}

fn main() {
    let data1 = WithOption {
        id: 1,
        name: Some("test".to_string()),
        count: Some(42),
    };
    
    let ref1 = data1.as_deep_ref();
    assert_eq!(ref1.id, 1);
    assert_eq!(ref1.name, Some("test"));
    assert_eq!(ref1.count, Some(42));
    
    let data2 = WithOption {
        id: 2,
        name: None,
        count: None,
    };
    
    let ref2 = data2.as_deep_ref();
    assert_eq!(ref2.id, 2);
    assert_eq!(ref2.name, None);
    assert_eq!(ref2.count, None);
}