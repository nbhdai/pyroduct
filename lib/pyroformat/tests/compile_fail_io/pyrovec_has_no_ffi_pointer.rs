//! §2: only a *sealed* buffer may cross FFI. The ABI is `PyroRefPtr`, and an
//! unsealed `PyroVec` cannot produce one — enforced by the type system, not by
//! convention.
//!
//! (`PyroVec` does deref to `[u8]`, so a bare `*const u8` to the payload is
//! reachable. That is not the ABI: nothing in the FFI surface accepts one, so
//! it cannot be mistaken for a message pointer.)
use pyroformat::buf::{PyroRefPtr, PyroVec};

fn main() {
    let v = PyroVec::with_capacity(16);
    let _: PyroRefPtr = v.as_ptr();
}
