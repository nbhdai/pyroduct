use arrow_scalars::{DeepRef, FromRow, ToRow};

#[test]
fn compile_base() {
    #[derive(FromRow, DeepRef, ToRow)]
    struct Address {
        street: String,
        zip: u32,
    }
    #[derive(FromRow, DeepRef, ToRow)]
    struct KitchenSink {
        id: u32,
        username: String,
        score: i32,
        address: Address,
        i32_vec: Vec<i32>,
        u64_vec: Vec<u64>,
        f64_vec: Vec<f64>,
        bool_vec: Vec<bool>,
    }

    let data = KitchenSink {
        id: 42,
        username: "alice".to_string(),
        score: 100,
        address: Address {
            street: "123 Main St".to_string(),
            zip: 12345,
        },
        i32_vec: vec![1, 2, 3],
        u64_vec: vec![100, 200],
        f64_vec: vec![1.1, 2.2],
        bool_vec: vec![true, false, true],
    };

    data.as_deep_ref();
}
