use pyroduct::{
    CapturedError,
    ffi::guest::{deserialize_input, execute_safe, serialize_output, serialize_result},
    format::{
        Bridgeable, BridgeableResult, HasReceiver, PyroVec, PyroView, Receiver, header::{DataStatus, PyroHeader}
    },
    magma,
    panic::register_ffi_panic_hook,
};

// --- Test Structures ---

#[magma(derive(Debug, Clone, PartialEq))]
#[derive(Debug, Clone, PartialEq)]
struct UserData {
    id: u32,
    payload: String,
}

#[magma(derive(Debug, Clone, PartialEq))]
#[derive(Debug, Clone, PartialEq)]
struct UserError {
    code: u16,
    msg: String,
}

// =============================================================================
// execute_safe
// =============================================================================

#[test]
fn test_execute_safe_success() {
    let vec = unsafe {
        PyroView::from_ptr(execute_safe(
            || {
                serialize_output(UserData {
                    id: 101,
                    payload: "Success".into(),
                })
            },
            0,
        ))
        .unwrap()
    };

    assert_eq!(vec.status(), Ok(DataStatus::RkyvValid));
    let typed = UserData::expose(vec).expect("Should parse as UserData");
    assert_eq!(typed.id, 101);
    assert_eq!(typed.payload.as_str(), "Success");
}

#[test]
fn test_execute_safe_large_payload() {
    let vec = unsafe {
        PyroView::from_ptr(execute_safe(
            || {
                serialize_output(UserData {
                    id: u32::MAX,
                    payload: "A".repeat(10_000),
                })
            },
            0,
        ))
        .unwrap()
    };

    assert_eq!(vec.status(), Ok(DataStatus::RkyvValid));
    let typed = UserData::expose(vec).unwrap();
    assert_eq!(typed.id, u32::MAX);
    assert_eq!(typed.payload.len(), 10_000);
}

#[test]
fn test_execute_safe_catches_panic() {
    register_ffi_panic_hook();

    let vec = unsafe {
        PyroView::from_ptr(execute_safe(
            || -> PyroVec {
                panic!("Intentional test panic");
            },
            0,
        ))
        .unwrap()
    };

    assert_eq!(vec.status(), Ok(DataStatus::CodeError));
    let slice = vec.as_slice();
    let error: CapturedError = serde_json::from_slice(slice).expect("Should be CapturedError");
    assert!(error.message.contains("Intentional test panic"));
}

#[test]
fn test_execute_safe_panic_captures_location() {
    register_ffi_panic_hook();

    let vec = unsafe {
        PyroView::from_ptr(execute_safe(
            || -> PyroVec {
                panic!("Location test");
            },
            0,
        ))
        .unwrap()
    };

    let error: CapturedError = serde_json::from_slice(vec.as_slice()).unwrap();
    assert!(error.file.contains("ffi_safety.rs"));
    assert!(error.line > 0);
}

#[test]
fn test_execute_safe_panic_then_success() {
    register_ffi_panic_hook();

    // First: panic
    let r1 = unsafe {
        PyroView::from_ptr(execute_safe(
            || -> PyroVec {
                panic!("boom");
            },
            0,
        ))
        .unwrap()
    };
    assert_eq!(r1.status(), Ok(DataStatus::CodeError));

    // Second: success — state should be clean
    let r2 = unsafe {
        PyroView::from_ptr(execute_safe(
            || {
                serialize_output(UserData {
                    id: 999,
                    payload: "Recovery".into(),
                })
            },
            0,
        ))
        .unwrap()
    };
    assert_eq!(r2.status(), Ok(DataStatus::RkyvValid));
    let typed = UserData::expose(r2).unwrap();
    assert_eq!(typed.id, 999);
}

#[test]
fn test_execute_safe_multiple_sequential() {
    for i in 0..10u32 {
        let vec = unsafe {
            PyroView::from_ptr(execute_safe(
                || {
                    serialize_output(UserData {
                        id: i,
                        payload: format!("Call {i}"),
                    })
                },
                0,
            ))
            .unwrap()
        };
        assert_eq!(vec.status(), Ok(DataStatus::RkyvValid));
        let typed = UserData::expose(vec).unwrap();
        assert_eq!(typed.id, i);
    }
}

// =============================================================================
// serialize_output
// =============================================================================

#[test]
fn test_serialize_output_struct() {
    let vec = serialize_output(UserData {
        id: 42,
        payload: "hello".into(),
    });
    assert_eq!(vec.status(), Ok(DataStatus::RkyvValid));

    let typed = UserData::expose(vec.view()).unwrap();
    assert_eq!(typed.id, 42);
    assert_eq!(typed.payload.as_str(), "hello");
}

#[test]
fn test_serialize_output_empty_string() {
    let vec = serialize_output(UserData {
        id: 0,
        payload: String::new(),
    });
    assert_eq!(vec.status(), Ok(DataStatus::RkyvValid));

    let typed = UserData::expose(vec.view()).unwrap();
    assert_eq!(typed.id, 0);
    assert!(typed.payload.is_empty());
}

#[test]
fn test_serialize_output_primitive() {
    let vec = serialize_output(42u64);
    assert_eq!(vec.status(), Ok(DataStatus::RkyvValid));

    let typed = u64::expose(vec.view()).expect("Should parse u64");
    assert_eq!(*typed, 42);
}

#[test]
fn test_serialize_output_vec() {
    let vec = serialize_output(vec![1u32, 2, 3, 4, 5]);
    assert_eq!(vec.status(), Ok(DataStatus::RkyvValid));

    let typed = Vec::<u32>::expose(vec.view()).unwrap();
    let mut receiver = typed.receiver();
    assert_eq!(receiver.receive(&typed).unwrap(), vec![1, 2, 3, 4, 5]);
}

#[test]
fn test_serialize_output_option_some() {
    let vec = serialize_output(Some("hello".to_string()));
    assert_eq!(vec.status(), Ok(DataStatus::RkyvValid));

    let typed = Option::<String>::expose(vec.view()).unwrap();
    let mut receiver = typed.receiver();
    assert_eq!(receiver.receive(&typed).unwrap(), Some("hello".to_string()));
}

#[test]
fn test_serialize_output_option_none() {
    let vec = serialize_output(Option::<String>::None);
    assert_eq!(vec.status(), Ok(DataStatus::RkyvValid));

    let typed = Option::<String>::expose(vec.view()).unwrap();
    let mut receiver = typed.receiver();
    assert_eq!(receiver.receive(&typed).unwrap(), None);
}

// =============================================================================
// serialize_result
// =============================================================================

#[test]
fn test_serialize_result_ok() {
    let vec = serialize_result::<UserData, UserError>(Ok(UserData {
        id: 200,
        payload: "ok".into(),
    }));
    assert_eq!(vec.status(), Ok(DataStatus::RkyvValid));

    let typed = <Result<UserData, UserError>>::expose(vec.view()).unwrap().unwrap();
    assert_eq!(typed.id, 200);
    assert_eq!(typed.payload.as_str(), "ok");
}

#[test]
fn test_serialize_result_err() {
    let vec = serialize_result::<UserData, UserError>(Err(UserError {
        code: 404,
        msg: "not found".into(),
    }));
    assert_eq!(vec.status(), Ok(DataStatus::RkyvError));

    let typed = <Result<UserData, UserError>>::expose(vec.view())
        .unwrap()
        .unwrap_err();
    assert_eq!(typed.code, 404);
    assert_eq!(typed.msg.as_str(), "not found");
}

// =============================================================================
// deserialize_input
// =============================================================================

#[test]
fn test_deserialize_input_roundtrip() {
    let original = UserData {
        id: 77,
        payload: "roundtrip".into(),
    };
    let shipped = original.ship().unwrap();
    let view_ptr = shipped.view().into_ptr();

    let recovered: UserData = deserialize_input(view_ptr).expect("Should deserialize");
    assert_eq!(recovered, original);
}

#[test]
fn test_deserialize_input_primitive() {
    let shipped = 123u32.ship().unwrap();
    let view_ptr = shipped.view().into_ptr();

    let recovered: u32 = deserialize_input(view_ptr).expect("Should deserialize u32");
    assert_eq!(recovered, 123);
}

// =============================================================================
// Roundtrip: serialize_output → from_raw → expose → receive
// =============================================================================

#[test]
fn test_full_roundtrip_user_data() {
    let original = UserData {
        id: 12345,
        payload: "roundtrip test".into(),
    };

    let vec = unsafe {
        PyroView::from_ptr(execute_safe(|| serialize_output(original.clone()), 0)).unwrap()
    };
    assert_eq!(vec.status(), Ok(DataStatus::RkyvValid));

    let typed = UserData::expose(vec).unwrap();
    let mut receiver = typed.receiver();
    let recovered = receiver.receive(&typed).unwrap();
    assert_eq!(original, recovered);
}

#[test]
fn test_full_roundtrip_user_error() {
    let original = UserError {
        code: 500,
        msg: "Internal error".into(),
    };

    let vec = unsafe {
        PyroView::from_ptr(execute_safe(|| serialize_output(original.clone()), 0)).unwrap()
    };
    assert_eq!(vec.status(), Ok(DataStatus::RkyvValid));

    let typed = UserError::expose(vec).unwrap();
    let mut receiver = typed.receiver();
    let recovered = receiver.receive(&typed).unwrap();
    assert_eq!(original, recovered);
}

// =============================================================================
// Panic hook idempotency
// =============================================================================

#[test]
fn test_panic_hook_idempotent() {
    register_ffi_panic_hook();
    register_ffi_panic_hook();
    register_ffi_panic_hook();

    let vec = unsafe {
        PyroView::from_ptr(execute_safe(
            || -> PyroVec {
                panic!("After multiple registrations");
            },
            0,
        ))
        .unwrap()
    };
    assert_eq!(vec.status(), Ok(DataStatus::CodeError));
}
