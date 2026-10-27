//! §4: the invariant the WASM story depends on — a payload slice cannot outlive
//! its `&mut self` borrow, so the host can never hold a live slice across a
//! reallocation (or, later, across a call into the guest).
use pyroformat::buf::PyroVec;

fn main() {
    let mut v = PyroVec::with_capacity(16);
    let slice = v.payload_mut();
    v.push(1); // reallocates; `slice` must already be dead here
    let _ = slice[0];
}
