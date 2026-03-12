//! Test FromRow with Vec fields (mapped to &[T])

use pyroduct::format::{DeepRef, FromRow, PyroRow, PyroValue, RefFromRow, ToRow};

#[derive(FromRow, RefFromRow, DeepRef, ToRow)]
struct WithVec {
    i32_vec: Vec<i32>,
    u64_vec: Vec<u64>,
    f64_vec: Vec<f64>,
    bool_vec: Vec<bool>,
}

fn main() {
    let row = PyroRow::from([
        ("i32_vec", PyroValue::from(&[10i32, 20, 30][..])),
        ("u64_vec", PyroValue::from(&[10u64, 20, 30][..])),
        ("f64_vec", PyroValue::from(&[1.1f64, 2.2, 3.3][..])),
        ("bool_vec", PyroValue::from(&[true, false, true][..])),
    ]);

    let s = WithVecRef::try_from(&row).unwrap();

    // Should be slices, not Vec
    assert_eq!(s.i32_vec, &[10, 20, 30]);
    assert_eq!(s.f64_vec, &[1.1, 2.2, 3.3]);

    // Verify it's actually a slice
    let _: &[i32] = s.i32_vec;
    let _: &[f64] = s.f64_vec;
    let _: &[u64] = s.u64_vec;
    let _: &[bool] = s.bool_vec;
}
