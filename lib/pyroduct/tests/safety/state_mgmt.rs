use pyroduct::CapIdentity;
use pyroduct::capability_host::ffi::*;
use pyroduct::errors::{FfiError, Phase, PyroductError};
use pyroduct::host::class::{CapClass, CapFunction, CapabilityInit, CapabilityReset, ClassState};
use std::cell::RefCell;
use std::ffi::c_void;
use std::ptr;

// ============================================================================
// Mock FFI Functions & Thread-Local State
// ============================================================================

// We use thread_local variables instead of global Atomics to ensure that
// tests running in parallel do not interfere with each other's state.
#[derive(Default, Clone, Copy, Debug)]
struct MockCounters {
    init: u32,
    drop: u32,
    reset: u32,
}

thread_local! {
    static COUNTERS: RefCell<MockCounters> = RefCell::new(MockCounters::default());
}

fn reset_mock_counters() {
    COUNTERS.with(|c| *c.borrow_mut() = MockCounters::default());
}

fn get_mock_counters() -> MockCounters {
    COUNTERS.with(|c| *c.borrow())
}

// Mock state struct
#[repr(C)]
struct MockState {
    value: u32,
}

// Sync init that returns a valid state pointer
unsafe extern "C" fn mock_sync_init(_config_ptr: *const u8, _config_len: usize) -> FfiInitResult {
    COUNTERS.with(|c| c.borrow_mut().init += 1);
    let state = Box::new(MockState { value: 42 });
    FfiInitResult::ok(Box::into_raw(state) as *mut c_void)
}

// Sync init that returns an error
unsafe extern "C" fn mock_sync_init_error(
    _config_ptr: *const u8,
    _config_len: usize,
) -> FfiInitResult {
    let ffi_error = FfiError::DeserializationFailed("Init failed (Mock)".to_string(), Phase::Init);
    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&ffi_error)
        .unwrap()
        .into_vec();

    let mut bytes = bytes;
    let (ptr, len, cap) = (bytes.as_mut_ptr(), bytes.len(), bytes.capacity());
    std::mem::forget(bytes);
    FfiInitResult::err(COutput { ptr, len, cap })
}

// Sync drop
unsafe extern "C" fn mock_sync_drop(state: *mut c_void) {
    COUNTERS.with(|c| c.borrow_mut().drop += 1);
    if !state.is_null() {
        let _ = unsafe { Box::from_raw(state as *mut MockState) };
    }
}

// Sync reset
unsafe extern "C" fn mock_sync_reset(state: *mut c_void) -> FfiResult {
    COUNTERS.with(|c| c.borrow_mut().reset += 1);
    if state.is_null() {
        return FfiResult::full_err(COutput {
            ptr: ptr::null(),
            len: 0,
            cap: 0,
        });
    }
    let state = unsafe { &mut *(state as *mut MockState) };
    state.value = 0;
    FfiResult::ok_null()
}

// Sync reset that returns error
unsafe extern "C" fn mock_sync_reset_error(_state: *mut c_void) -> FfiResult {
    let ffi_error =
        FfiError::DeserializationFailed("Reset failed (Mock)".to_string(), Phase::Reset);
    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&ffi_error)
        .unwrap()
        .into_vec();

    let mut bytes = bytes;
    let (ptr, len, cap) = (bytes.as_mut_ptr(), bytes.len(), bytes.capacity());
    std::mem::forget(bytes);
    FfiResult::full_err(COutput { ptr, len, cap })
}

// Mock capability functions
unsafe extern "C" fn mock_sync_function(
    _client_ptr: *const u8,
    _client_len: usize,
    _input_ptr: *const u8,
    _input_len: usize,
    state_ptr: *mut c_void,
) -> FfiResult {
    if state_ptr.is_null() {
        return FfiResult::full_err(COutput {
            ptr: ptr::null(),
            len: 0,
            cap: 0,
        });
    }
    let state = unsafe { &*(state_ptr as *const MockState) };
    let result = state.value.to_string().into_bytes();
    let (ptr, len, cap) = (result.as_ptr(), result.len(), result.capacity());
    std::mem::forget(result);
    FfiResult::ok(COutput { ptr, len, cap })
}

// ============================================================================
// Test Helpers
// ============================================================================

fn create_mock_function_export(cap_name: &str, func_name: &str) -> FunctionExport<'static> {
    FunctionExport {
        capability: cap_name.as_ptr(),
        capability_len: cap_name.len(),
        name: func_name.as_ptr(),
        name_len: func_name.len(),
        func: Function::Sync(mock_sync_function),
    }
}

fn create_mock_class_export(
    exports: &[FunctionExport<'static>],
    init_fn: ClassInitFn<'static>,
    drop_fn: ClassDropFn,
    reset_fn: ClassResetFn<'static>,
) -> ClassExport<'static> {
    ClassExport {
        ptr: exports.as_ptr(),
        init: init_fn,
        drop: drop_fn,
        reset: reset_fn,
        len: exports.len(),
    }
}

fn create_test_identity() -> CapIdentity {
    CapIdentity::from(std::path::Path::new("/test/capability.so"))
}

// ============================================================================
// CapFunction Tests
// ============================================================================

#[test]
fn test_cap_function_creation() {
    reset_mock_counters();
    let cap_name = "test_cap";
    let func_name = "test_func";
    let export = create_mock_function_export(cap_name, func_name);
    let ident = create_test_identity();

    let func = CapFunction::new(&export, &ident).expect("Should succeed");

    assert_eq!(func.cap_name, cap_name);
    assert_eq!(func.func_name, func_name);
}

#[test]
fn test_cap_function_with_null_pointers_returns_error() {
    reset_mock_counters();
    let export = FunctionExport {
        capability: ptr::null(),
        capability_len: 0,
        name: ptr::null(),
        name_len: 0,
        func: Function::Sync(mock_sync_function),
    };
    let ident = create_test_identity();

    let result = CapFunction::new(&export, &ident);
    assert!(result.is_err());
}

// ============================================================================
// CapClass Tests
// ============================================================================

#[test]
fn test_cap_class_creation() {
    reset_mock_counters();
    let cap_name = "test_cap";
    let func_name = "test_func";
    let export = create_mock_function_export(cap_name, func_name);
    let exports = vec![export];

    let class_export = create_mock_class_export(
        &exports,
        ClassInitFn::Sync(mock_sync_init),
        ClassDropFn::Sync(mock_sync_drop),
        ClassResetFn::Sync(mock_sync_reset),
    );

    let ident = create_test_identity();
    let class = CapClass::new(&ident, &class_export).expect("Should create class");

    assert_eq!(class.ident, ident);
    assert_eq!(class.imports.len(), 1);
    assert_eq!(class.imports[0].cap_name, cap_name);
    assert_eq!(class.imports[0].func_name, func_name);
}

#[test]
fn test_cap_class_with_multiple_functions() {
    reset_mock_counters();
    let exports = vec![
        create_mock_function_export("cap1", "func1"),
        create_mock_function_export("cap2", "func2"),
        create_mock_function_export("cap3", "func3"),
    ];

    let class_export = create_mock_class_export(
        &exports,
        ClassInitFn::Sync(mock_sync_init),
        ClassDropFn::Sync(mock_sync_drop),
        ClassResetFn::Sync(mock_sync_reset),
    );

    let ident = create_test_identity();
    let class = CapClass::new(&ident, &class_export).expect("Should create class");

    assert_eq!(class.imports.len(), 3);
    assert_eq!(class.imports[0].func_name, "func1");
    assert_eq!(class.imports[1].func_name, "func2");
    assert_eq!(class.imports[2].func_name, "func3");
}

// ============================================================================
// CapClass::init Tests
// ============================================================================

#[tokio::test]
async fn test_cap_class_sync_init_success() {
    reset_mock_counters();
    let exports = vec![create_mock_function_export("test", "func")];
    let class_export = create_mock_class_export(
        &exports,
        ClassInitFn::Sync(mock_sync_init),
        ClassDropFn::Sync(mock_sync_drop),
        ClassResetFn::Sync(mock_sync_reset),
    );

    let ident = create_test_identity();
    let class = CapClass::new(&ident, &class_export).unwrap();

    let init_result = class.init(None).unwrap();
    let state = init_result.await.unwrap();

    assert_eq!(get_mock_counters().init, 1);
    assert!(!state.ptr.is_null());
    assert_eq!(unsafe { (*(state.ptr as *const MockState)).value }, 42);
}

#[tokio::test]
async fn test_cap_class_sync_init_with_config() {
    reset_mock_counters();
    let exports = vec![create_mock_function_export("test", "func")];
    let class_export = create_mock_class_export(
        &exports,
        ClassInitFn::Sync(mock_sync_init),
        ClassDropFn::Sync(mock_sync_drop),
        ClassResetFn::Sync(mock_sync_reset),
    );

    let ident = create_test_identity();
    let class = CapClass::new(&ident, &class_export).unwrap();

    let config = serde_json::json!({"key": "value"});
    let init_result = class.init(Some(&config)).unwrap();
    let state = init_result.await.unwrap();

    assert_eq!(get_mock_counters().init, 1);
    assert!(!state.ptr.is_null());
}

#[tokio::test]
async fn test_cap_class_sync_init_error() {
    reset_mock_counters();
    let exports = vec![create_mock_function_export("test", "func")];
    let class_export = create_mock_class_export(
        &exports,
        ClassInitFn::Sync(mock_sync_init_error),
        ClassDropFn::Sync(mock_sync_drop),
        ClassResetFn::Sync(mock_sync_reset),
    );

    let ident = create_test_identity();
    let class = CapClass::new(&ident, &class_export).unwrap();

    let init_result = class.init(None);

    assert!(init_result.is_err());
}

#[tokio::test]
async fn test_cap_class_null_init() {
    reset_mock_counters();
    let exports = vec![create_mock_function_export("test", "func")];
    let class_export = create_mock_class_export(
        &exports,
        ClassInitFn::Null,
        ClassDropFn::Null,
        ClassResetFn::Null,
    );

    let ident = create_test_identity();
    let class = CapClass::new(&ident, &class_export).unwrap();

    let init_result = class.init(None).unwrap();
    let state = init_result.await.unwrap();

    assert_eq!(get_mock_counters().init, 0);
    assert!(state.ptr.is_null());
}

// ============================================================================
// ClassState Tests
// ============================================================================

#[tokio::test]
async fn test_class_state_reset_sync_success() {
    reset_mock_counters();
    let state_ptr = Box::into_raw(Box::new(MockState { value: 42 })) as *mut c_void;
    let mut state = ClassState {
        ident: create_test_identity(),
        ptr: state_ptr,
        reset_fn: ClassResetFn::Sync(mock_sync_reset),
        destroy_fn: ClassDropFn::Sync(mock_sync_drop),
    };

    let reset_result = state.reset();
    let result = reset_result.await;

    assert!(result.is_ok());
    assert_eq!(get_mock_counters().reset, 1);
    assert_eq!(unsafe { (*(state.ptr as *const MockState)).value }, 0);
}

#[tokio::test]
async fn test_class_state_reset_sync_error() {
    reset_mock_counters();
    let state_ptr = Box::into_raw(Box::new(MockState { value: 42 })) as *mut c_void;
    let mut state = ClassState {
        ident: create_test_identity(),
        ptr: state_ptr,
        reset_fn: ClassResetFn::Sync(mock_sync_reset_error),
        destroy_fn: ClassDropFn::Sync(mock_sync_drop),
    };

    let reset_result = state.reset();
    let result = reset_result.await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_class_state_reset_null() {
    reset_mock_counters();
    let state_ptr = Box::into_raw(Box::new(MockState { value: 42 })) as *mut c_void;
    let mut state = ClassState {
        ident: create_test_identity(),
        ptr: state_ptr,
        reset_fn: ClassResetFn::Null,
        destroy_fn: ClassDropFn::Sync(mock_sync_drop),
    };

    let reset_result = state.reset();
    let result = reset_result.await;

    assert!(result.is_ok());
    assert_eq!(get_mock_counters().reset, 0);
}

#[test]
fn test_class_state_drop() {
    reset_mock_counters();
    let state_ptr = Box::into_raw(Box::new(MockState { value: 42 })) as *mut c_void;
    {
        let _state = ClassState {
            ident: create_test_identity(),
            ptr: state_ptr,
            reset_fn: ClassResetFn::Sync(mock_sync_reset),
            destroy_fn: ClassDropFn::Sync(mock_sync_drop),
        };
        // State dropped here
    }

    assert_eq!(get_mock_counters().drop, 1);
}

#[test]
fn test_class_state_drop_with_null_ptr() {
    reset_mock_counters();
    {
        let _state = ClassState {
            ident: create_test_identity(),
            ptr: ptr::null_mut(),
            reset_fn: ClassResetFn::Null,
            destroy_fn: ClassDropFn::Sync(mock_sync_drop),
        };
        // State dropped here
    }

    assert_eq!(get_mock_counters().drop, 0);
}

#[test]
fn test_class_state_drop_with_null_fn() {
    reset_mock_counters();
    let state_ptr = Box::into_raw(Box::new(MockState { value: 42 })) as *mut c_void;
    {
        let _state = ClassState {
            ident: create_test_identity(),
            ptr: state_ptr,
            reset_fn: ClassResetFn::Null,
            destroy_fn: ClassDropFn::Null,
        };
        // State dropped here
    }

    assert_eq!(get_mock_counters().drop, 0);
    // Manual cleanup to prevent leak in test
    unsafe { drop(Box::from_raw(state_ptr as *mut MockState)) };
}

// ============================================================================
// CapabilityInit Future Tests
// ============================================================================

#[tokio::test]
async fn test_capability_init_sync_polling() {
    reset_mock_counters();
    let state_ptr = Box::into_raw(Box::new(MockState { value: 99 })) as *mut c_void;
    let init = CapabilityInit::Sync {
        ident: create_test_identity(),
        reset_fn: ClassResetFn::Sync(mock_sync_reset),
        state: Some(state_ptr),
        destroy_fn: ClassDropFn::Sync(mock_sync_drop),
    };

    let state = init.await.unwrap();

    assert_eq!(state.ptr, state_ptr);
    assert_eq!(unsafe { (*(state.ptr as *const MockState)).value }, 99);
}

#[tokio::test]
#[should_panic(expected = "Double await!")]
async fn test_capability_init_sync_double_poll_panics() {
    reset_mock_counters();
    let state_ptr = Box::into_raw(Box::new(MockState { value: 99 })) as *mut c_void;
    let mut init = CapabilityInit::Sync {
        ident: create_test_identity(),
        reset_fn: ClassResetFn::Sync(mock_sync_reset),
        state: Some(state_ptr),
        destroy_fn: ClassDropFn::Sync(mock_sync_drop),
    };

    // First poll should succeed
    use std::future::Future;
    use std::pin::Pin;
    use std::task::{Context, Poll, Waker};

    let waker: Waker = futures::task::noop_waker();
    let mut cx = Context::from_waker(&waker);

    let mut pinned = Pin::new(&mut init);
    let poll1 = pinned.as_mut().poll(&mut cx);
    assert!(matches!(poll1, Poll::Ready(Ok(_))));

    // Second poll should panic
    let _poll2 = pinned.poll(&mut cx);
}

#[tokio::test]
async fn test_capability_init_null() {
    reset_mock_counters();
    let init = CapabilityInit::Null(create_test_identity());
    let state = init.await.unwrap();

    assert!(state.ptr.is_null());
}

// ============================================================================
// CapabilityReset Future Tests
// ============================================================================

#[tokio::test]
async fn test_capability_reset_sync_or_null_success() {
    reset_mock_counters();
    let reset = CapabilityReset::SyncOrNull(create_test_identity(), Some(Ok(())));
    let result = reset.await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_capability_reset_sync_or_null_error() {
    reset_mock_counters();
    let error = PyroductError::from_infrastructure("Test error");
    let reset = CapabilityReset::SyncOrNull(create_test_identity(), Some(Err(error)));
    let result = reset.await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_capability_reset_sync_or_null_double_poll() {
    reset_mock_counters();
    let mut reset = CapabilityReset::SyncOrNull(create_test_identity(), Some(Ok(())));

    use std::future::Future;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    let waker = futures::task::noop_waker();
    let mut cx = Context::from_waker(&waker);

    // First poll
    let mut pinned = Pin::new(&mut reset);
    let poll1 = pinned.as_mut().poll(&mut cx);
    assert!(matches!(poll1, Poll::Ready(Ok(_))));

    // Second poll should return error
    let poll2 = pinned.poll(&mut cx);
    assert!(matches!(poll2, Poll::Ready(Err(_))));
}

// ============================================================================
// Edge Cases and Safety Tests
// ============================================================================

#[test]
fn test_class_state_is_send() {
    fn assert_send<T: Send>() {}
    assert_send::<ClassState>();
}

#[tokio::test]
async fn test_multiple_states_independent_lifecycle() {
    reset_mock_counters();

    let state1_ptr = Box::into_raw(Box::new(MockState { value: 1 })) as *mut c_void;
    let state2_ptr = Box::into_raw(Box::new(MockState { value: 2 })) as *mut c_void;

    let state1 = ClassState {
        ident: create_test_identity(),
        ptr: state1_ptr,
        reset_fn: ClassResetFn::Sync(mock_sync_reset),
        destroy_fn: ClassDropFn::Sync(mock_sync_drop),
    };

    let mut state2 = ClassState {
        ident: create_test_identity(),
        ptr: state2_ptr,
        reset_fn: ClassResetFn::Sync(mock_sync_reset),
        destroy_fn: ClassDropFn::Sync(mock_sync_drop),
    };

    // Reset state2
    let reset_result = state2.reset();
    reset_result.await.unwrap();

    // State1 should be unaffected
    assert_eq!(unsafe { (*(state1.ptr as *const MockState)).value }, 1);
    // State2 should be reset
    assert_eq!(unsafe { (*(state2.ptr as *const MockState)).value }, 0);
    assert_eq!(get_mock_counters().reset, 1);

    drop(state1);
    assert_eq!(get_mock_counters().drop, 1);

    drop(state2);
    assert_eq!(get_mock_counters().drop, 2);
}

#[tokio::test]
async fn test_cap_class_init_called_multiple_times() {
    reset_mock_counters();

    let exports = vec![create_mock_function_export("test", "func")];
    let class_export = create_mock_class_export(
        &exports,
        ClassInitFn::Sync(mock_sync_init),
        ClassDropFn::Sync(mock_sync_drop),
        ClassResetFn::Sync(mock_sync_reset),
    );

    let ident = create_test_identity();
    let class = CapClass::new(&ident, &class_export).unwrap();

    // Call init multiple times
    let init1 = class.init(None).unwrap();
    let state1 = init1.await.unwrap();

    let init2 = class.init(None).unwrap();
    let state2 = init2.await.unwrap();

    assert_eq!(get_mock_counters().init, 2);
    assert_ne!(state1.ptr, state2.ptr); // Different instances
}
