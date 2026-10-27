//! `PyroCart::get()` cannot hand out a `&T::Ref<'_>` that outlives the cart
//! itself — `docs/deep-ref-plan.md` §7.

use pyroformat::buf::{PyroCart, PyroVec};
use pyroformat::buf::header::schema_hash;
use pyroformat::{Layout, Writer, Reader};

#[derive(Layout, Writer, Reader, Debug, Clone, PartialEq)]
struct Foo {
    name: String,
}

fn bad() -> &'static FooRef<'static> {
    let mut vec = PyroVec::zeroed(0);
    vec.extend_from_slice(&[0u8; 16]);
    let buf = vec.seal(schema_hash(&<Foo as pyroformat::Layout>::META, <Foo as pyroformat::Layout>::INODES));
    let cart = PyroCart::<Foo>::from_buf(buf).unwrap();
    cart.get()
}

fn main() {}
