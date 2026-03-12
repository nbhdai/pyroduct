use pyroduct::{
    format::{Bridgeable, DeepRef, ToRow},
    magma,
};

#[magma]
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

    let row = s.to_row();
    let _row_reference: SimpleStructRef = row.try_into().unwrap();
    let row = s.to_row();
    let restored: SimpleStruct = row.try_into().unwrap();
    let _reference = restored.as_deep_ref();
}
