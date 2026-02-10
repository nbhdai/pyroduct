use pyro_vec::{bridgeable, Bridgeable};

#[bridgeable]
struct DataContainer {
    items: Vec<u8>,
    labels: Vec<String>,
}

fn main() {
    let d = DataContainer {
        items: vec![1, 2, 3],
        labels: vec!["a".to_string(), "b".to_string()],
    };
    let _ = d.ship();
}