use bridge_vec::{bridgeable, Bridgeable};

#[bridgeable]
enum Status {
    Active,
    Inactive,
    Pending { reason: String },
}

fn main() {
    let s = Status::Pending {
        reason: "waiting".to_string(),
    };
    let _ = s;
}