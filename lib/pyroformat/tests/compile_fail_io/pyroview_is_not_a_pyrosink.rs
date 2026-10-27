//! `PyroSink` is for a handle that can still be written into (today
//! `LocalMemSink`, over an exclusive `PyroBuf`). `PyroBuf::view()` consumes the
//! `PyroBuf` into a `PyroView` — a shared, read-only handle — and that
//! handle must not become writable through the same trait.
use pyroformat::buf::{PyroSink, PyroVec};

fn main() {
    let v = PyroVec::with_capacity(16);
    let buf = v.seal(0);
    let mut view = buf.view();
    let _ = PyroSink::payload_mut(&mut view);
}
