//! §2 / fix (6): `PyroVec` grows, and growth reallocates, which would dangle a
//! `PyroView`'s refcount pointer. The hazard is designed out rather than
//! documented around: an unsealed buffer simply cannot produce a view.
use pyroformat::buf::PyroVec;

fn main() {
    let v = PyroVec::with_capacity(16);
    let _ = v.view();
}
