//! Generics are not currently supported

use pyro_vec::ToRow;

#[derive(ToRow)]
struct GenericStruct<T> {
    value: T,
}

fn main() {}