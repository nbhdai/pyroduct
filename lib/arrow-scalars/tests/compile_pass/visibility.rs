//! Test that visibility modifiers are preserved

use arrow_scalars::{FromRow, ToRow, DeepRef};

#[derive(FromRow, ToRow, DeepRef)]
pub struct PublicStruct {
    pub public_field: i32,
    private_field: String,
}

#[derive(FromRow, ToRow, DeepRef)]
struct PrivateStruct {
    field: i32,
}

mod inner {
    use arrow_scalars::{FromRow, ToRow, DeepRef};
    
    #[derive(FromRow, ToRow, DeepRef)]
    pub(crate) struct CrateVisible {
        pub(super) field: i32,
    }
}

fn main() {
    // Test that generated Ref structs respect visibility
    let data = PublicStruct {
        public_field: 42,
        private_field: "test".to_string(),
    };
    
    let _ref = data.as_deep_ref();
    
    // Should compile - PublicStructRef is public
    let _: PublicStructRef = _ref;
}