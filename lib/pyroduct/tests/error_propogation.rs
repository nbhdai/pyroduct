//! Tests for error propagation across FFI boundaries
use pyroduct::CapIdentity;
use pyroduct::arrow_scalars::{ArrowRow, ArrowValue};
use pyroduct::capability::safe_call::{empty_call, i_call, sc_call, sci_call};
use pyroduct::capability::safe_io::{get_client_state, get_input, make_output};
use pyroduct::capability_host::ffi::{COutput, FfiResult};
use pyroduct::errors::{ArchivedFfiError, FfiError, Phase};
use pyroduct::host::ffi_bridge::ExecutionResultBridge;
use pyroduct::module_capability::panic::register_ffi_panic_hook;
use std::ffi::c_void;
use std::path::Path;

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Debug, PartialEq, Clone)]
struct TestClient {
    id: u64,
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Debug, PartialEq, Clone)]
struct TestInput {
    command: String,
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Debug, PartialEq, Clone)]
struct TestOutput {
    result: String,
}

struct TestState {
    initialized: bool,
}

fn create_test_ident() -> CapIdentity {
    CapIdentity::from(Path::new("/test/error_prop.so"))
}

fn deserialize_ffi_error(result: FfiResult) -> FfiError {
    assert!(result.tag != 0, "Expected error result");
    let bytes = unsafe {
        Vec::from_raw_parts(
            result.output.ptr as *mut u8,
            result.output.len,
            result.output.cap,
        )
    };
    rkyv::from_bytes::<FfiError, rkyv::rancor::Error>(&bytes).unwrap()
}

#[test]
fn test_sci_call_null_state_pointer() {
    register_ffi_panic_hook();

    let client = TestClient { id: 1 };
    let input = TestInput {
        command: "test".into(),
    };

    let client_bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&client)
        .unwrap()
        .into_vec();
    let input_bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&input)
        .unwrap()
        .into_vec();

    let result = sci_call::<TestState, TestClient, TestInput, TestOutput, _>(
        client_bytes.as_ptr(),
        client_bytes.len(),
        input_bytes.as_ptr(),
        input_bytes.len(),
        std::ptr::null_mut(), // NULL state pointer
        |_state, _client, _input| TestOutput {
            result: "ok".into(),
        },
    );

    let error = deserialize_ffi_error(result);
    match error {
        FfiError::NullPointer(Phase::State) => {}
        _ => panic!("Expected NullPointer(State), got {:?}", error),
    }
}

#[test]
fn test_sci_call_null_client_pointer() {
    register_ffi_panic_hook();

    let mut state = TestState { initialized: true };
    let input = TestInput {
        command: "test".into(),
    };
    let input_bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&input)
        .unwrap()
        .into_vec();

    let result = sci_call::<TestState, TestClient, TestInput, TestOutput, _>(
        std::ptr::null(), // NULL client pointer
        0,
        input_bytes.as_ptr(),
        input_bytes.len(),
        &mut state as *mut _ as *mut c_void,
        |_state, _client, _input| TestOutput {
            result: "ok".into(),
        },
    );

    let error = deserialize_ffi_error(result);
    match error {
        FfiError::NullPointer(Phase::Input) => {}
        _ => panic!("Expected NullPointer(Client), got {:?}", error),
    }
}

#[test]
fn test_sci_call_null_input_pointer() {
    register_ffi_panic_hook();

    let mut state = TestState { initialized: true };
    let client = TestClient { id: 1 };
    let client_bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&client)
        .unwrap()
        .into_vec();

    let result = sci_call::<TestState, TestClient, TestInput, TestOutput, _>(
        client_bytes.as_ptr(),
        client_bytes.len(),
        std::ptr::null(), // NULL input pointer
        0,
        &mut state as *mut _ as *mut c_void,
        |_state, _client, _input| TestOutput {
            result: "ok".into(),
        },
    );

    let error = deserialize_ffi_error(result);
    match error {
        FfiError::NullPointer(Phase::Input) => {}
        _ => panic!("Expected NullPointer(Input), got {:?}", error),
    }
}

#[test]
fn test_sci_call_invalid_client_bytes() {
    register_ffi_panic_hook();

    let mut state = TestState { initialized: true };
    let garbage_client = vec![0xFF, 0x00, 0xAB, 0xCD];
    let input = TestInput {
        command: "test".into(),
    };
    let input_bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&input)
        .unwrap()
        .into_vec();

    let result = sci_call::<TestState, TestClient, TestInput, TestOutput, _>(
        garbage_client.as_ptr(),
        garbage_client.len(),
        input_bytes.as_ptr(),
        input_bytes.len(),
        &mut state as *mut _ as *mut c_void,
        |_state, _client, _input| TestOutput {
            result: "ok".into(),
        },
    );

    let error = deserialize_ffi_error(result);
    match error {
        FfiError::ValidationFailed(_, Phase::Input) | FfiError::DeserializationFailed(_, _) => {}
        _ => panic!("Expected validation/deserialization error, got {:?}", error),
    }
}

#[test]
fn test_sci_call_logic_panic_propagation() {
    register_ffi_panic_hook();

    let mut state = TestState { initialized: true };
    let client = TestClient { id: 1 };
    let input = TestInput {
        command: "crash".into(),
    };
    let client_bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&client)
        .unwrap()
        .into_vec();
    let input_bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&input)
        .unwrap()
        .into_vec();

    let result = sci_call::<TestState, TestClient, TestInput, TestOutput, _>(
        client_bytes.as_ptr(),
        client_bytes.len(),
        input_bytes.as_ptr(),
        input_bytes.len(),
        &mut state as *mut _ as *mut c_void,
        |_state, _client, input| {
            if input.command == "crash" {
                panic!("Intentional panic in capability logic");
            }
            TestOutput {
                result: "ok".into(),
            }
        },
    );

    let error = deserialize_ffi_error(result);
    match error {
        FfiError::CapabilityLogicPanicked(info) => {
            assert!(info.message.contains("Intentional panic"));
        }
        _ => panic!("Expected CapabilityLogicPanicked, got {:?}", error),
    }
}

#[test]
fn test_i_call_success() {
    register_ffi_panic_hook();

    let input = TestInput {
        command: "hello".into(),
    };
    let input_bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&input)
        .unwrap()
        .into_vec();

    let result = i_call::<TestInput, TestOutput, _>(
        std::ptr::null(),
        0,
        input_bytes.as_ptr(),
        input_bytes.len(),
        std::ptr::null_mut(),
        |input| TestOutput {
            result: format!("received: {}", input.command),
        },
    );

    assert_eq!(result.tag, 0, "Should succeed");
}

#[test]
fn test_empty_call_success() {
    register_ffi_panic_hook();

    let result = empty_call::<TestOutput, _>(
        std::ptr::null(),
        0,
        std::ptr::null(),
        0,
        std::ptr::null_mut(),
        || TestOutput {
            result: "empty call worked".into(),
        },
    );

    assert_eq!(result.tag, 0, "Should succeed");
}

#[test]
fn test_empty_call_panic() {
    register_ffi_panic_hook();

    let result = empty_call::<TestOutput, _>(
        std::ptr::null(),
        0,
        std::ptr::null(),
        0,
        std::ptr::null_mut(),
        || -> TestOutput { panic!("empty call panic") },
    );

    let error = deserialize_ffi_error(result);
    match error {
        FfiError::CapabilityLogicPanicked(info) => {
            assert!(info.message.contains("empty call panic"));
        }
        _ => panic!("Expected CapabilityLogicPanicked, got {:?}", error),
    }
}

#[test]
fn test_error_roundtrip_through_bridge() {
    register_ffi_panic_hook();

    // Create an error on the "plugin" side
    let result = empty_call::<TestOutput, _>(
        std::ptr::null(),
        0,
        std::ptr::null(),
        0,
        std::ptr::null_mut(),
        || -> TestOutput { panic!("Bridge test panic") },
    );

    // Process through the "host" bridge
    let ident = create_test_ident();
    let host_result = unsafe { ExecutionResultBridge::from_ffi(result, &ident) };

    match host_result {
        Err(e) => {
            let msg = e.to_string();
            assert!(msg.contains("Bridge test panic"));
            assert!(msg.contains("logic panic"));
        }
        Ok(_) => panic!("Expected error from bridge"),
    }
}

#[test]
fn test_sc_call_zero_length_client() {
    register_ffi_panic_hook();

    let mut state = TestState { initialized: true };
    let client_bytes = vec![1, 2, 3]; // Non-null but will pass zero length

    let result = sc_call::<TestState, TestClient, TestOutput, _>(
        client_bytes.as_ptr(),
        0, // Zero length
        std::ptr::null(),
        0,
        &mut state as *mut _ as *mut c_void,
        |_state, _client| TestOutput {
            result: "ok".into(),
        },
    );

    let error = deserialize_ffi_error(result);
    match error {
        FfiError::ZeroLength(Phase::Input) => {}
        _ => panic!("Expected ZeroLength, got {:?}", error),
    }
}

#[test]
fn test_make_output_serialization_success() {
    let output = TestOutput {
        result: "test output".into(),
    };

    let result = unsafe { make_output(&output) };
    assert_eq!(result.tag, 0);
    assert!(!result.output.ptr.is_null());
    assert!(result.output.len > 0);

    // Cleanup
    unsafe {
        Vec::from_raw_parts(
            result.output.ptr as *mut u8,
            result.output.len,
            result.output.cap,
        );
    }
}
