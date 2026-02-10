use pyro_vec::{
    PyroVec, Bridgeable, CapturedError, bridgeable,
    ffi::{execute_safe, register_ffi_panic_hook},
    format::{HasReceiver, Receiver},
    header::{PyroHeader, DataStatus},
};

// --- Test Structures ---

#[bridgeable]
#[derive(Debug, Clone, PartialEq)]
struct UserData {
    id: u32,
    payload: String,
}

#[bridgeable]
#[derive(Debug, Clone, PartialEq)]
struct UserError {
    code: u16,
    msg: String,
}

// --- Success Path Tests ---

#[test]
fn test_execute_safe_success_path() {
    let result = unsafe {
        PyroVec::from_raw(execute_safe(|| UserData {
            id: 101,
            payload: "Success".to_string(),
        }))
        .unwrap()
    };

    assert_eq!(result.status(), Ok(DataStatus::RkyvValid));

    let typed = UserData::expose(result).expect("Should parse as UserData");
    // Zero-copy access
    assert_eq!(typed.id, 101);
    assert_eq!(typed.payload.as_str(), "Success");
}

#[test]
fn test_execute_safe_with_complex_data() {
    let result = unsafe {
        PyroVec::from_raw(execute_safe(|| UserData {
            id: u32::MAX,
            payload: "A".repeat(10000),
        }))
        .unwrap()
    };

    assert_eq!(result.status(), Ok(DataStatus::RkyvValid));

    let typed = UserData::expose(result).expect("Should parse large payload");
    assert_eq!(typed.id, u32::MAX);
    assert_eq!(typed.payload.len(), 10000);
}

#[test]
fn test_execute_safe_empty_string() {
    let result = unsafe {
        PyroVec::from_raw(execute_safe(|| UserData {
            id: 0,
            payload: String::new(),
        }))
        .unwrap()
    };

    assert_eq!(result.status(), Ok(DataStatus::RkyvValid));

    let typed = UserData::expose(result).expect("Should parse empty payload");
    assert_eq!(typed.id, 0);
    assert!(typed.payload.is_empty());
}

// --- Panic Safety Tests ---

#[test]
fn test_execute_safe_catches_panic() {
    register_ffi_panic_hook();

    let result = unsafe {
        PyroVec::from_raw(execute_safe(|| -> UserData {
            panic!("Intentional test panic");
        }))
        .unwrap()
    };

    assert_eq!(result.status(), Ok(DataStatus::CodeError));

    // Parse as JSON to verify error structure
    let slice = result.as_slice();
    let error: CapturedError =
        serde_json::from_slice(slice).expect("Should deserialize as CapturedError");

    assert!(error.message.contains("Intentional test panic"));
    assert!(!error.file.is_empty());
    assert!(error.line > 0);
}

#[test]
fn test_execute_safe_catches_panic_with_string_payload() {
    register_ffi_panic_hook();

    let result = unsafe {
        PyroVec::from_raw(execute_safe(|| -> UserData {
            panic!("Owned string panic message");
        }))
        .unwrap()
    };

    assert_eq!(result.status(), Ok(DataStatus::CodeError));

    let slice = result.as_slice();
    let error: CapturedError =
        serde_json::from_slice(slice).expect("Should deserialize as CapturedError");

    assert!(error.message.contains("Owned string panic message"));
}

#[test]
fn test_execute_safe_catches_panic_with_format() {
    register_ffi_panic_hook();

    let value = 42;
    let result = unsafe {
        PyroVec::from_raw(execute_safe(|| -> UserData {
            panic!("Formatted panic: value={}", value);
        }))
        .unwrap()
    };

    assert_eq!(result.status(), Ok(DataStatus::CodeError));

    let slice = result.as_slice();
    let error: CapturedError =
        serde_json::from_slice(slice).expect("Should deserialize as CapturedError");

    assert!(error.message.contains("value=42"));
}

#[test]
fn test_panic_location_captured() {
    register_ffi_panic_hook();

    let result = unsafe {
        PyroVec::from_raw(execute_safe(|| -> UserData {
            panic!("Location test");
        }))
        .unwrap()
    };

    let slice = result.as_slice();
    let error: CapturedError = serde_json::from_slice(slice).unwrap();

    // File should contain the test file path
    assert!(error.file.contains("ffi_safety.rs"));
    // Line should be reasonable (not 0)
    assert!(error.line > 0);
    // Column should be captured
    assert!(error.column > 0);
}

// --- Multiple Sequential Calls ---

#[test]
fn test_multiple_successful_calls() {
    for i in 0..10 {
        let result = unsafe {
            PyroVec::from_raw(execute_safe(|| UserData {
                id: i,
                payload: format!("Call {}", i),
            }))
            .unwrap()
        };

        assert_eq!(result.status(), Ok(DataStatus::RkyvValid));
        let typed = UserData::expose(result).unwrap();
        assert_eq!(typed.id, i);
    }
}

#[test]
fn test_panic_then_success() {
    register_ffi_panic_hook();

    // First call panics
    let result1 = unsafe {
        PyroVec::from_raw(execute_safe(|| -> UserData {
            panic!("First call panic");
        }))
        .unwrap()
    };

    assert_eq!(result1.status(), Ok(DataStatus::CodeError));

    // Second call succeeds - state should be clean
    let result2 = unsafe {
        PyroVec::from_raw(execute_safe(|| UserData {
            id: 999,
            payload: "Recovery".to_string(),
        }))
        .unwrap()
    };
    assert_eq!(result2.status(), Ok(DataStatus::RkyvValid));

    let typed = UserData::expose(result2).unwrap();
    assert_eq!(typed.id, 999);
}

#[test]
fn test_alternating_panic_and_success() {
    register_ffi_panic_hook();

    for i in 0..5 {
        if i % 2 == 0 {
            let result = unsafe {
                PyroVec::from_raw(execute_safe(|| -> UserData {
                    panic!("Panic iteration {}", i);
                }))
                .unwrap()
            };
            assert_eq!(result.status(), Ok(DataStatus::CodeError));
        } else {
            let result = unsafe {
                PyroVec::from_raw(execute_safe(|| UserData {
                    id: i,
                    payload: "Success".to_string(),
                }))
                .unwrap()
            };
            assert_eq!(result.status(), Ok(DataStatus::RkyvValid));
        }
    }
}

// --- Edge Cases ---

#[test]
fn test_execute_safe_with_unit_type() {
    // Test that unit type () works with PyroVec::from_raw(execute_safe
    let result = unsafe { PyroVec::from_raw(execute_safe(|| ())) }.unwrap();
    assert_eq!(result.status(), Ok(DataStatus::RkyvValid));
}

#[test]
fn test_execute_safe_with_primitive() {
    let result = unsafe { PyroVec::from_raw(execute_safe(|| 42u64)) }.unwrap();
    assert_eq!(result.status(), Ok(DataStatus::RkyvValid));

    let typed = u64::expose(result).expect("Should parse u64");
    assert_eq!(*typed, 42);
}

#[test]
fn test_execute_safe_with_vec() {
    let result = unsafe { PyroVec::from_raw(execute_safe(|| vec![1u32, 2, 3, 4, 5])) }.unwrap();
    assert_eq!(result.status(), Ok(DataStatus::RkyvValid));

    let typed = Vec::<u32>::expose(result).expect("Should parse Vec");
    let mut receiver = typed.receiver();
    assert_eq!(receiver.receive(&typed).unwrap(), vec![1, 2, 3, 4, 5]);
}

#[test]
fn test_execute_safe_with_option_some() {
    let result =
        unsafe { PyroVec::from_raw(execute_safe(|| Some("hello".to_string()))) }.unwrap();
    assert_eq!(result.status(), Ok(DataStatus::RkyvValid));

    let typed = Option::<String>::expose(result).expect("Should parse Option");
    let mut receiver = typed.receiver();
    assert_eq!(receiver.receive(&typed).unwrap(), Some("hello".to_string()));
}

#[test]
fn test_execute_safe_with_option_none() {
    let result = unsafe { PyroVec::from_raw(execute_safe(|| Option::<String>::None)) }.unwrap();
    assert_eq!(result.status(), Ok(DataStatus::RkyvValid));

    let typed = Option::<String>::expose(result).expect("Should parse Option");
    let mut receiver = typed.receiver();
    assert_eq!(receiver.receive(&typed).unwrap(), None);
}

// --- Thread Safety ---

#[test]
fn test_panic_hook_idempotent() {
    // Calling register multiple times should be safe
    register_ffi_panic_hook();
    register_ffi_panic_hook();
    register_ffi_panic_hook();

    let result = unsafe {
        PyroVec::from_raw(execute_safe(|| -> UserData {
            panic!("After multiple registrations");
        }))
        .unwrap()
    };

    assert_eq!(result.status(), Ok(DataStatus::CodeError));
}

// --- Roundtrip Tests ---

#[test]
fn test_user_data_roundtrip() {
    let original = UserData {
        id: 12345,
        payload: "roundtrip test".to_string(),
    };

    let result = unsafe { PyroVec::from_raw(execute_safe(|| original.clone())) }.unwrap();
    assert_eq!(result.status(), Ok(DataStatus::RkyvValid));

    let typed = UserData::expose(result).unwrap();
    let mut receiver = typed.receiver();
    let recovered = receiver.receive(&typed).unwrap();

    assert_eq!(original, recovered);
}

#[test]
fn test_user_error_roundtrip() {
    let original = UserError {
        code: 500,
        msg: "Internal error".to_string(),
    };

    let result = unsafe { PyroVec::from_raw(execute_safe(|| original.clone())) }.unwrap();
    assert_eq!(result.status(), Ok(DataStatus::RkyvValid));

    let typed = UserError::expose(result).unwrap();
    let mut receiver = typed.receiver();
    let recovered = receiver.receive(&typed).unwrap();

    assert_eq!(original, recovered);
}
