use pyro_vec::{bridgeable, Bridgeable};

#[bridgeable]
struct UnitLike;

fn main() {
    let u = UnitLike;
    let _ = u.ship();
}