use pyroduct::format::{
    get_view, PyroBuf, PyroVec, PyroVecPtr, PyroView,
};
use pyroduct::format::header::{DataStatus, PyroData, PyroHeader, PyroParser};

#[test]
fn test_vec_basic_operations() {
    let mut vec = PyroVec::with_capacity(10);
    vec.push(1);
    vec.push(2);
    vec.push(3);

    assert_eq!(vec.len(), 3);
    assert_eq!(&*vec, &[1, 2, 3]);

    vec.extend_from_slice(&[4, 5, 6]);
    assert_eq!(vec.len(), 6);
    assert_eq!(&*vec, &[1, 2, 3, 4, 5, 6]);

    vec.clear();
    assert_eq!(vec.len(), 0);
    assert!(vec.as_slice().is_empty());
}

#[test]
fn test_vec_growth() {
    // Start with small capacity
    let mut vec = PyroVec::with_capacity(1);
    let initial_cap = vec.capacity();

    // Push enough to trigger growth
    for i in 0..100 {
        vec.push(i as u8);
    }

    assert_eq!(vec.len(), 100);
    assert!(vec.capacity() >= 100);
    assert!(vec.capacity() > initial_cap);

    for i in 0..100 {
        assert_eq!(vec[i], i as u8);
    }
}

#[test]
fn test_vec_extend_growth() {
    let mut vec = PyroVec::with_capacity(1);
    let data = vec![1u8; 100];
    vec.extend_from_slice(&data);

    assert_eq!(vec.len(), 100);
    assert_eq!(&*vec, &data[..]);
}

#[test]
fn test_vec_into_raw_roundtrip() {
    let mut vec = PyroVec::with_capacity(10);
    vec.extend_from_slice(b"hello raw");
    let original_len = vec.len();

    let raw = vec.into_raw();

    let recovered = unsafe { PyroVec::from_raw(raw) }.expect("Should recover vec");
    assert_eq!(recovered.len(), original_len);
    assert_eq!(&*recovered, b"hello raw");
}

#[test]
fn test_vec_from_raw_null() {
    // PyroVecPtr fields are private, so we transmute null pointers to construct one
    let raw_null: PyroVecPtr =
        unsafe { std::mem::transmute::<[usize; 3], PyroVecPtr>([0, 0, 0]) };
    let res = unsafe { PyroVec::from_raw(raw_null) };
    assert!(res.is_err());
}

#[test]
fn test_vec_to_buf_roundtrip() {
    let mut vec = PyroVec::with_capacity(10);
    vec.extend_from_slice(b"hello buf");

    let buf = vec.into_buf();
    assert_eq!(buf.len(), 9);
    assert_eq!(&*buf, b"hello buf");
}

#[test]
fn test_buf_into_raw_roundtrip() {
    let mut vec = PyroVec::with_capacity(10);
    vec.extend_from_slice(b"hello raw buf");
    let buf = vec.into_buf();

    let raw = buf.into_raw();
    let recovered = unsafe { PyroBuf::from_raw(raw) }.expect("Should recover buf");
    assert_eq!(&*recovered, b"hello raw buf");
}

#[test]
fn test_vec_clone() {
    let mut vec = PyroVec::with_capacity(10);
    vec.extend_from_slice(b"clone me");

    let cloned = vec.clone();
    assert_eq!(cloned.len(), vec.len());
    assert_eq!(&*cloned, &*vec);

    // Ensure it's a deep copy
    let mut vec_mut = vec;
    vec_mut.push(0);
    assert_ne!(vec_mut.len(), cloned.len());
}

#[test]
fn test_vec_ok() {
    let vec = PyroVec::ok();
    assert_eq!(vec.len(), 0);
    assert_eq!(vec.status(), Ok(DataStatus::Empty));
}

#[test]
fn test_view_from_vec() {
    let mut vec = PyroVec::with_capacity(10);
    vec.extend_from_slice(b"view test");

    let view: PyroView = vec.view();
    assert_eq!(view.len(), 9);
    assert_eq!(&*view, b"view test");
}

#[test]
fn test_view_from_ptr_roundtrip() {
    let mut vec = PyroVec::with_capacity(10);
    vec.extend_from_slice(b"ptr test");
    let view_ptr = vec.view().ptr();

    let recovered_view = unsafe { PyroView::from_ptr(view_ptr) }.expect("Should recover view");
    assert_eq!(&*recovered_view, b"ptr test");
}

#[test]
fn test_get_view_valid() {
    let mut vec = PyroVec::with_capacity(32);
    vec.extend_from_slice(b"hello world");

    let full_len = 16 + PyroParser::HEADER_SIZE + vec.len();
    let memory = unsafe {
        // as_raw_slice starts at offset 16 into the allocation; subtract to get PyroInner start
        let inner_ptr = vec.view().as_raw_slice().as_ptr().sub(PyroParser::HEADER_SIZE) as *const u8;
        std::slice::from_raw_parts(inner_ptr, full_len)
    };

    let view = get_view(memory, 0).expect("Should create view");
    assert_eq!(view.len(), 11);
    assert_eq!(&*view, b"hello world");
}

#[test]
fn test_get_view_misaligned() {
    let mut vec = PyroVec::with_capacity(32);
    vec.extend_from_slice(b"hello world");

    let full_len = 16 + PyroParser::HEADER_SIZE + vec.len();
    let memory = unsafe {
        let inner_ptr = vec.view().as_raw_slice().as_ptr().sub(PyroParser::HEADER_SIZE) as *const u8;
        std::slice::from_raw_parts(inner_ptr, full_len)
    };

    // Offset 1 is misaligned
    let res = get_view(memory, 1);
    assert!(res.is_err());
}

#[test]
fn test_get_view_too_small() {
    let memory = vec![0u8; 20];
    let res = get_view(&memory, 0);
    assert!(res.is_err());
}

#[test]
fn test_slice_representations_public() {
    let mut vec = PyroVec::with_capacity(32);
    vec.extend_from_slice(b"hello pyroduct");

    assert_eq!(vec.len(), 14);
    assert_eq!(&*vec, b"hello pyroduct");
    assert_eq!(vec.as_slice(), b"hello pyroduct");

    let raw = vec.as_raw_slice();
    assert_eq!(raw.len(), PyroParser::HEADER_SIZE + 14);
    assert_eq!(&raw[PyroParser::HEADER_SIZE..], b"hello pyroduct");
}
