use bridge_vec::bridgeable;

#[bridgeable]
struct UnitLike;

fn main() {
    let u = UnitLike;
    let _ = u;
}