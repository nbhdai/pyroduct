//! Generics are not currently supported

use arrow_scalars::ToRow;

#[derive(ToRow)]
struct GenericStruct<T> {
    value: T,
}

fn main() {}