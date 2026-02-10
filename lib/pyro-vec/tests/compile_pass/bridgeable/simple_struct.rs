use pyro_vec::{bridgeable, Bridgeable};

#[bridgeable]
struct SimpleStruct {
    id: u32,
    name: String,
}

fn main() {
    let s = SimpleStruct {
        id: 1,
        name: "waiting".to_string(),
    };
    let _ = s.ship();
}