//! `PyroVec` is not a valid `PyroCart` owner — only sealed (`PyroBuf`) or
//! refcounted (`PyroView`) handles have the stable address `PyroCart`
//! requires (it's growable and can reallocate) — `docs/deep-ref-plan.md` §7.

use pyroformat::buf::{PyroCart, PyroVec};
use pyroformat::{Layout, Writer, Reader};

#[derive(Layout, Writer, Reader, Debug, Clone, PartialEq)]
struct Foo {
    name: String,
}

fn bad() {
    let vec = PyroVec::zeroed(16);
    let _cart = PyroCart::<Foo>::from_buf(vec);
}

fn main() {}
