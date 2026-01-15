//! Generics are not currently supported

use arrow_scalars::{FromRow, DeepRef};

#[derive(FromRow, DeepRef)]
struct GenericStruct<T> {
    value: T,
}

fn main() {}