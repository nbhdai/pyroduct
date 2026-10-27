//! `PyroCart<T>` tests — `docs/deep-ref-plan.md` §7.

use pyroformat::buf::{CartOwner, PyroCart, PyroVec};
use pyroformat::{Layout, Reader, Realize, Writer};

#[derive(Layout, Writer, Reader, Debug, Clone, PartialEq)]
#[pyro(strict_read)]
struct Point {
    label: String,
    x: i32,
    y: i32,
}

fn sealed_buf(data: &Point) -> pyroformat::buf::PyroBuf {
    data.encode().unwrap()
}

fn assert_send_sync<T: Send + Sync>() {}

#[test]
fn cart_is_send_and_sync() {
    assert_send_sync::<PyroCart<Point>>();
}

#[test]
fn cart_outlives_its_buffer_expression() {
    let data = Point {
        label: "origin".to_string(),
        x: 3,
        y: 4,
    };
    let cart = {
        let buf = sealed_buf(&data);
        PyroCart::<Point>::from_buf(buf).unwrap()
    };
    assert_eq!(Realize::<Point>::realize(cart.get()), data);
}

#[test]
fn cart_from_view_survives_originating_buf() {
    let data = Point {
        label: "view".to_string(),
        x: -1,
        y: 2,
    };
    let buf = sealed_buf(&data);
    let view = buf.view(); // `buf` is consumed here
    let cart = PyroCart::<Point>::from_view(view).unwrap();
    assert_eq!(Realize::<Point>::realize(cart.get()), data);
}

#[test]
fn cart_refcount_reaches_zero_exactly_once_on_drop() {
    let data = Point {
        label: "rc".to_string(),
        x: 0,
        y: 0,
    };
    let buf = sealed_buf(&data);
    let view = buf.view();
    assert_eq!(view.ref_count(), 1);

    let cart = PyroCart::<Point>::from_view(view).unwrap();
    let view_ref_count = match cart.owner() {
        CartOwner::View(v) => v.ref_count(),
        CartOwner::Buf(_) => panic!("expected a View-backed cart"),
    };
    assert_eq!(view_ref_count, 1);

    let cloned = cart.try_clone().expect("View-backed cart should clone");
    let after_clone = match cloned.owner() {
        CartOwner::View(v) => v.ref_count(),
        CartOwner::Buf(_) => panic!("expected a View-backed cart"),
    };
    assert_eq!(after_clone, 2);

    drop(cloned);
    let after_drop = match cart.owner() {
        CartOwner::View(v) => v.ref_count(),
        CartOwner::Buf(_) => panic!("expected a View-backed cart"),
    };
    assert_eq!(after_drop, 1);
}

#[test]
fn buf_backed_cart_cannot_be_cloned() {
    let data = Point {
        label: "no-clone".to_string(),
        x: 5,
        y: 6,
    };
    let buf = sealed_buf(&data);
    let cart = PyroCart::<Point>::from_buf(buf).unwrap();
    assert!(cart.try_clone().is_none());
}

#[test]
fn cart_schema_mismatch_is_rejected() {
    #[derive(Layout, Writer, Reader, Debug, Clone, PartialEq)]
    #[pyro(strict_read)]
    struct Other {
        a: u32,
        b: u32,
    }

    let mut vec = PyroVec::zeroed(0);
    vec.extend_from_slice(&[0u8; 8]);
    let buf = vec.seal(<Other as Layout>::SCHEMA_HASH);

    assert!(PyroCart::<Point>::from_buf(buf).is_err());
}
