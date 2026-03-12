//! Generics are not currently supported

use pyroduct::format::ToRow;

#[derive(ToRow)]
struct GenericStruct<T> {
    value: T,
}

fn main() {}
