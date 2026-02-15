//! Generics are not currently supported

use pyroduct::ToRow;

#[derive(ToRow)]
struct GenericStruct<T> {
    value: T,
}

fn main() {}