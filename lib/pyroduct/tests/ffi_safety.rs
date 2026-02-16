use pyroduct::{
    Bridgeable, BridgeableResult, CapturedError, PyroVec, PyroViewPtr, bridgeable,
    ffi::{
        PyroObject, PyroRefObjectPtr,
        guest::{
            panic_wrap::execute_safe,
            safe_call::{empty_call, i_call, sc_call, sci_call_result},
        },
    },
    format::{HasReceiver, Receiver},
    header::{DataStatus, PyroHeader},
    panic::register_ffi_panic_hook,
};
use std::ptr;

// --- Test Structures ---

#[bridgeable(derive(Debug, Clone, PartialEq))]
#[derive(Debug, Clone, PartialEq)]
struct UserData {
    id: u32,
    payload: String,
}

#[bridgeable(derive(Debug, Clone, PartialEq))]
#[derive(Debug, Clone, PartialEq)]
struct UserError {
    code: u16,
    msg: String,
}

// --- Success Path Tests ---

#[tracing_test::traced_test]
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

#[tracing_test::traced_test]
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

#[tracing_test::traced_test]
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

#[tracing_test::traced_test]
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

#[tracing_test::traced_test]
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

#[tracing_test::traced_test]
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

#[tracing_test::traced_test]
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

#[tracing_test::traced_test]
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

#[tracing_test::traced_test]
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

#[tracing_test::traced_test]
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

#[tracing_test::traced_test]
#[test]
fn test_execute_safe_with_unit_type() {
    // Test that unit type () works with PyroVec::from_raw(execute_safe
    let result = unsafe { PyroVec::from_raw(execute_safe(|| ())) }.unwrap();
    assert_eq!(result.status(), Ok(DataStatus::RkyvValid));
}

#[tracing_test::traced_test]
#[test]
fn test_execute_safe_with_primitive() {
    let result = unsafe { PyroVec::from_raw(execute_safe(|| 42u64)) }.unwrap();
    assert_eq!(result.status(), Ok(DataStatus::RkyvValid));

    let typed = u64::expose(result).expect("Should parse u64");
    assert_eq!(*typed, 42);
}

#[tracing_test::traced_test]
#[test]
fn test_execute_safe_with_vec() {
    let result = unsafe { PyroVec::from_raw(execute_safe(|| vec![1u32, 2, 3, 4, 5])) }.unwrap();
    assert_eq!(result.status(), Ok(DataStatus::RkyvValid));

    let typed = Vec::<u32>::expose(result).expect("Should parse Vec");
    let mut receiver = typed.receiver();
    assert_eq!(receiver.receive(&typed).unwrap(), vec![1, 2, 3, 4, 5]);
}

#[tracing_test::traced_test]
#[test]
fn test_execute_safe_with_option_some() {
    let result = unsafe { PyroVec::from_raw(execute_safe(|| Some("hello".to_string()))) }.unwrap();
    assert_eq!(result.status(), Ok(DataStatus::RkyvValid));

    let typed = Option::<String>::expose(result).expect("Should parse Option");
    let mut receiver = typed.receiver();
    assert_eq!(receiver.receive(&typed).unwrap(), Some("hello".to_string()));
}

#[tracing_test::traced_test]
#[test]
fn test_execute_safe_with_option_none() {
    let result = unsafe { PyroVec::from_raw(execute_safe(|| Option::<String>::None)) }.unwrap();
    assert_eq!(result.status(), Ok(DataStatus::RkyvValid));

    let typed = Option::<String>::expose(result).expect("Should parse Option");
    let mut receiver = typed.receiver();
    assert_eq!(receiver.receive(&typed).unwrap(), None);
}

// --- Thread Safety ---

#[tracing_test::traced_test]
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

#[tracing_test::traced_test]
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

#[tracing_test::traced_test]
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

// --- Mock State and Functions for safe_call testing ---

struct MockServer {
    value: u32,
}

// Implement standard traits required for safe execution
unsafe impl Send for MockServer {}
unsafe impl Sync for MockServer {}

impl MockServer {
    fn test_sci(&self, _client: UserData, input: UserData) -> Result<UserData, UserError> {
        if input.id == 0 {
            Err(UserError {
                code: 400,
                msg: "Invalid ID".into(),
            })
        } else {
            Ok(UserData {
                id: self.value + input.id,
                payload: input.payload,
            })
        }
    }

    fn test_sc(&self, client: UserData) -> UserData {
        UserData {
            id: self.value,
            payload: client.payload,
        }
    }
}

fn standalone_i(input: UserData) -> UserData {
    UserData {
        id: input.id * 2,
        payload: "standalone".into(),
    }
}

fn standalone_empty() -> u32 {
    42
}

// --- helper to create a Mock Object ---
fn create_mock_state(val: u32) -> PyroObject {
    unsafe extern "C" fn dropper(ptr: *mut std::ffi::c_void) {
        drop(unsafe { Box::from_raw(ptr as *mut MockServer) });
    }
    let state = Box::into_raw(Box::new(MockServer { value: val })) as *mut std::ffi::c_void;
    unsafe { PyroObject::new(state, dropper).unwrap() }
}

// --- safe_call Tests ---

#[tracing_test::traced_test]
#[test]
fn test_sci_call_success() {
    let state = create_mock_state(100);
    let client_vec = UserData {
        id: 1,
        payload: "C".into(),
    }
    .ship()
    .unwrap();
    let input_vec = UserData {
        id: 50,
        payload: "I".into(),
    }
    .ship()
    .unwrap();

    let res_ptr = sci_call_result::<MockServer, UserData, UserData, UserData, UserError, _>(
        state.ref_ptr(),
        client_vec.view().ptr(),
        input_vec.view().ptr(),
        |s, c, i| s.test_sci(c, i),
    );

    let result = unsafe { PyroVec::from_raw(res_ptr).unwrap() };
    assert_eq!(result.status(), Ok(DataStatus::RkyvValid));
    let typed = Result::<UserData, UserError>::expose(result)
        .unwrap()
        .unwrap();
    assert_eq!(typed.id, 150);
}

#[tracing_test::traced_test]
#[test]
fn test_sci_call_user_error() {
    let state = create_mock_state(100);
    let client_vec = UserData {
        id: 1,
        payload: "C".into(),
    }
    .ship()
    .unwrap();
    let input_vec = UserData {
        id: 0,
        payload: "Fail".into(),
    }
    .ship()
    .unwrap();

    let res_ptr = sci_call_result::<MockServer, UserData, UserData, UserData, UserError, _>(
        state.ref_ptr(),
        client_vec.view().ptr(),
        input_vec.view().ptr(),
        |s, c, i| s.test_sci(c, i),
    );

    let result = unsafe { PyroVec::from_raw(res_ptr).unwrap() };
    assert_eq!(result.status(), Ok(DataStatus::RkyvError));
    let typed = Result::<UserData, UserError>::expose(result)
        .unwrap()
        .unwrap_err();
    assert_eq!(typed.code, 400);
}

#[tracing_test::traced_test]
#[test]
fn test_sc_call_success() {
    let state = create_mock_state(500);
    let client_vec = UserData {
        id: 1,
        payload: "ClientData".into(),
    }
    .ship()
    .unwrap();

    let res_ptr = sc_call::<MockServer, UserData, UserData, _>(
        state.ref_ptr(),
        client_vec.view().ptr(),
        PyroViewPtr {
            ptr: ptr::null(),
            len: 0,
        },
        |s, c| s.test_sc(c),
    );

    let result = unsafe { PyroVec::from_raw(res_ptr).unwrap() };
    let typed = UserData::expose(result).unwrap();
    assert_eq!(typed.id, 500);
    assert_eq!(typed.payload.as_str(), "ClientData");
}

#[tracing_test::traced_test]
#[test]
fn test_i_call_success() {
    let input_vec = UserData {
        id: 21,
        payload: "Input".into(),
    }
    .ship()
    .unwrap();

    let res_ptr = i_call::<UserData, UserData, _>(
        PyroRefObjectPtr {
            state: ptr::null_mut(),
        },
        PyroViewPtr {
            ptr: ptr::null(),
            len: 0,
        },
        input_vec.view().ptr(),
        standalone_i,
    );

    let result = unsafe { PyroVec::from_raw(res_ptr).unwrap() };
    let typed = Result::<UserData, UserError>::expose(result)
        .unwrap()
        .unwrap();
    assert_eq!(typed.id, 42);
}

#[tracing_test::traced_test]
#[test]
fn test_empty_call_success() {
    let res_ptr = empty_call::<u32, _>(
        PyroRefObjectPtr {
            state: ptr::null_mut(),
        },
        PyroViewPtr {
            ptr: ptr::null(),
            len: 0,
        },
        PyroViewPtr {
            ptr: ptr::null(),
            len: 0,
        },
        standalone_empty,
    );

    let result = unsafe { PyroVec::from_raw(res_ptr).unwrap() };
    let typed = u32::expose(result).unwrap();
    assert_eq!(*typed, 42);
}

#[tracing_test::traced_test]
#[test]
fn test_safe_call_invalid_state_ptr() {
    let bad_state = PyroRefObjectPtr {
        state: ptr::null_mut(),
    };
    let client_vec = UserData {
        id: 1,
        payload: "x".into(),
    }
    .ship()
    .unwrap();

    let res_ptr = sc_call::<MockServer, UserData, UserData, _>(
        bad_state,
        client_vec.view().ptr(),
        PyroViewPtr {
            ptr: ptr::null(),
            len: 0,
        },
        |s, c| s.test_sc(c),
    );

    let result = unsafe { PyroVec::from_raw(res_ptr).unwrap() };
    assert_eq!(result.status(), Ok(DataStatus::CodeError)); // Panics on null state
}

#[tracing_test::traced_test]
#[test]
fn test_safe_call_deserialization_failure() {
    let state = create_mock_state(100);
    // Provide an empty/garbage view where valid data is expected
    let bad_view = PyroViewPtr {
        ptr: ptr::null(),
        len: 0,
    };

    let res_ptr = sc_call::<MockServer, UserData, UserData, _>(
        state.ref_ptr(),
        bad_view,
        PyroViewPtr {
            ptr: ptr::null(),
            len: 0,
        },
        |s, c| s.test_sc(c),
    );

    let result = unsafe { PyroVec::from_raw(res_ptr).unwrap() };
    assert_eq!(result.status(), Ok(DataStatus::PyroFfiFail)); // Failed to parse header
}
