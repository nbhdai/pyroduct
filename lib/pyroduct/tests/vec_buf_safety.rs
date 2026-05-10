use pyroduct::format::{PyroVec, PyroView};
use std::sync::Arc;
use std::thread;

#[test]
fn test_pyro_vec_basic_lifecycle() {
    {
        let mut vec = PyroVec::with_capacity(100);
        vec.extend_from_slice(b"hello world");
        assert_eq!(vec.as_slice(), b"hello world");
    } // Should drop and deallocate without crashing
}

#[test]
fn test_pyro_vec_growth_lifecycle() {
    {
        let mut vec = PyroVec::with_capacity(10);
        for i in 0..1000 {
            vec.push(i as u8);
        }
        assert_eq!(vec.len(), 1000);
    } // Should drop and deallocate without crashing
}

#[test]
fn test_pyro_view_ref_counting() {
    let vec = PyroVec::with_capacity(100);
    let view1 = vec.view();
    
    let view2 = view1.clone();
    let view3 = view2.clone();
    
    drop(view1);
    drop(view2);
    drop(view3);
}

#[test]
fn test_pyro_view_ptr_roundtrip() {
    let vec = PyroVec::with_capacity(100);
    let view = vec.view();
    let ptr = view.clone().into_ptr(); // ref count becomes 2
    
    let view_reconstructed = unsafe { PyroView::from_ptr(ptr).unwrap() };
    // ref count still 2 (from_ptr doesn't increment)
    
    assert_eq!(view.as_slice(), view_reconstructed.as_slice());
    
    drop(view);
    drop(view_reconstructed);
    // ref count should now be 0 (since we had 2 refs and dropped 2)
    // Wait, ptr() created a ref that is never dropped unless we reconstructed it.
    // Let's trace:
    // 1. vec.view() -> ref=1, view owned.
    // 2. view.ptr() -> ref=2, returns PyroViewPtr.
    // 3. from_ptr(ptr) -> ref=2, returns view_reconstructed.
    // 4. drop(view) -> ref=1.
    // 5. drop(view_reconstructed) -> ref=0 -> Free!
}

#[test]
fn test_pyro_view_multithreaded() {
    let mut vec = PyroVec::with_capacity(100);
    vec.extend_from_slice(b"shared data");
    let view = vec.view();
    let shared_view = Arc::new(view);
    
    let mut handles = vec![];
    for _ in 0..10 {
        let v = Arc::clone(&shared_view);
        handles.push(thread::spawn(move || {
            assert_eq!(v.as_slice(), b"shared data");
            let cloned = (*v).clone();
            assert_eq!(cloned.as_slice(), b"shared data");
            // cloned is dropped here
        }));
    }
    
    for h in handles {
        h.join().unwrap();
    }
}

#[test]
fn test_pyro_view_no_op_dropper_simulation() {
    // We can't easily simulate a no_op_dropper here because it's private.
    // But we can test that a regular view works.
    let vec = PyroVec::with_capacity(10);
    let view = vec.view();
    drop(view);
}

#[test]
fn test_pyro_vec_zero_capacity() {
    let mut vec = PyroVec::with_capacity(0);
    assert_eq!(vec.len(), 0);
    vec.push(1);
    assert_eq!(vec.len(), 1);
    assert_eq!(vec.as_slice(), &[1]);
}