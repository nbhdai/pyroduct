//! §2: `PyroRef<'a>` is lifetime-bound. It crosses FFI inward only — outward,
//! the lifetime would be a lie.
use pyroformat::buf::{PyroRef, PyroVec};

fn escape<'a>() -> PyroRef<'a> {
    let buf = PyroVec::with_capacity(16).seal(1);
    buf.as_ref()
}

fn main() {
    let _ = escape();
}
