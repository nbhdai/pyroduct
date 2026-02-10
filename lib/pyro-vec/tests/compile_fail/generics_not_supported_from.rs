//! Generics are not currently supported

use pyro_vec::{FromRow, DeepRef};

#[derive(FromRow, DeepRef)]
struct GenericStruct<T> {
    value: T,
}

fn main() {}