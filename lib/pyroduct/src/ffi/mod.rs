mod interface;
pub use interface::*;

#[cfg(feature = "host")]
pub mod host;

#[cfg(feature = "capability")]
pub mod guest;

use crate::format::format::UserHeaderValues;

impl UserHeaderValues for serde_json::Value {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ffi::host::ForeignClass;
    use crate::format::header::{PyroData, PyroHeader};
    use crate::format::vec_buf::PyroRefPtr;
    use crate::format::{PyroVec, PyroViewPtr};
    use crate::module::capability::create_log;
    use std::cell::RefCell;
    use std::ffi::c_void;
    use std::ptr;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    const NAME: &str = "TestName";

    /// Holds the actual flags for a specific test instance.
    /// This will be Boxed and passed as the `state` (*mut c_void) of the PyroObject.
    struct TestContext {
        drop_called: Arc<AtomicBool>,
        reset_called: Arc<AtomicBool>,
        register_called: Arc<AtomicBool>,
    }

    // Thread local storage to safely pass the state pointer into `mock_init`
    // without requiring argument threading. This works perfectly with parallel cargo tests.
    thread_local! {
        static NEXT_OBJ_STATE: RefCell<Option<*mut c_void>> = const { RefCell::new(None) };
    }

    unsafe extern "C" fn mock_dropper(ptr: *mut c_void) {
        if ptr.is_null() {
            return;
        }
        // Retake ownership of the box so it drops cleanly when this scope ends
        let ctx = unsafe { Box::from_raw(ptr as *mut TestContext) };
        ctx.drop_called.store(true, Ordering::SeqCst);
    }

    unsafe extern "C" fn mock_init(_: PyroRefPtr, object_id: u64) -> InitResult {
        // Retrieve the pointer pre-allocated by the test setup
        let state = NEXT_OBJ_STATE.with(|s| {
            s.borrow_mut()
                .take()
                .expect("mock_init called but no state was prepared in TLS")
        });

        InitResult {
            state: PyroObjectPtr {
                state,
                dropper: mock_dropper,
                object_id,
            },
            error: PyroVec::ok().view().into_ptr(),
        }
    }

    unsafe extern "C" fn mock_reset_sync(ptr: PyroRefObjectPtr) -> PyroViewPtr {
        if !ptr.state.is_null() {
            let ctx = unsafe { &*(ptr.state as *const TestContext) };
            ctx.reset_called.store(true, Ordering::SeqCst);
        }
        PyroVec::ok().view().into_ptr()
    }

    unsafe extern "C" fn mock_register_sync(
        ptr: PyroRefObjectPtr,
        _client: PyroRefPtr,
    ) -> PyroViewPtr {
        if !ptr.state.is_null() {
            let ctx = unsafe { &*(ptr.state as *const TestContext) };
            ctx.register_called.store(true, Ordering::SeqCst);
        }
        PyroVec::ok().view().into_ptr()
    }

    // ============================================================================
    // 3. Test Helper API
    // ============================================================================

    pub struct Mocks {
        pub drop_called: Arc<AtomicBool>,
        pub reset_called: Arc<AtomicBool>,
        pub register_called: Arc<AtomicBool>,

        /// The raw pointer to the TestContext.
        /// For PyroObject::new() tests, pass this directly.
        /// For ForeignClass tests, this is auto-injected via TLS when init is called.
        pub state_ptr: *mut c_void,

        // The function pointers to use in ClassExport
        pub init: ClassInitFn,
        pub dropper: ClassDropper,
        pub reset: ClassResetFn,
        pub register: ClientRegisterFn,
    }

    fn mocks() -> Mocks {
        let drop_called = Arc::new(AtomicBool::new(false));
        let reset_called = Arc::new(AtomicBool::new(false));
        let register_called = Arc::new(AtomicBool::new(false));

        let context = Box::new(TestContext {
            drop_called: drop_called.clone(),
            reset_called: reset_called.clone(),
            register_called: register_called.clone(),
        });

        let state_ptr = Box::into_raw(context) as *mut c_void;

        // Pre-load the TLS so if mock_init is called on this thread, it grabs this state
        NEXT_OBJ_STATE.with(|s| {
            *s.borrow_mut() = Some(state_ptr);
        });

        Mocks {
            drop_called,
            reset_called,
            register_called,
            state_ptr,
            init: ClassInitFn::Sync(mock_init),
            dropper: mock_dropper,
            reset: ClassResetFn::Sync(mock_reset_sync),
            register: ClientRegisterFn::Sync(mock_register_sync),
        }
    }

    // ============================================================================
    // 4. Refactored Tests
    // ============================================================================

    #[test]
    fn test_owned_lifecycle() {
        let m = mocks();

        // Pass m.state_ptr directly. The Mocks struct ensures `NEXT_OBJ_STATE` is set,
        // but for manual `PyroObject::new`, we just use the pointer.
        // Important: clear TLS to avoid leakage if we aren't using init
        NEXT_OBJ_STATE.with(|s| {
            *s.borrow_mut() = None;
        });

        let obj = unsafe { PyroObject::new(m.state_ptr, m.dropper, 0) }.unwrap();
        drop(obj);

        assert!(
            m.drop_called.load(Ordering::SeqCst),
            "Dropper should be called on drop"
        );
    }

    #[test]
    fn test_ref_ptr_does_not_drop() {
        let m = mocks();
        NEXT_OBJ_STATE.with(|s| {
            *s.borrow_mut() = None;
        }); // Manual creation, clear TLS

        let obj = unsafe { PyroObject::new(m.state_ptr, m.dropper, 0) }.unwrap();
        let ref_ptr = obj.ref_ptr();

        assert_eq!(ref_ptr.state, m.state_ptr);

        let _ = unsafe { PyroObjectRef::from_raw(ref_ptr) }.unwrap();

        assert!(!m.drop_called.load(Ordering::SeqCst));

        drop(obj);
        assert!(m.drop_called.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_foreign_class_reset() {
        let m = mocks();

        let export = ClassExport {
            name: NAME.as_ptr(),
            name_len: NAME.len(),
            ptr: ptr::null(),
            len: 0,
            init: m.init,   // Uses TLS to find m.state_ptr
            reset: m.reset, // Syncs with m.reset_called
            register: ClientRegisterFn::Null,
        };

        let class = Arc::new(
            unsafe { ForeignClass::from_export_inter("test".to_string(), None, export) }.unwrap(),
        );
        let log_channel = create_log(0, 0, 100);

        // Create Instance (calls mock_init, which claims m.state_ptr from TLS)
        let config = PyroVec::ok();
        let handle = class
            .create_instance(config.py_ref(), 0, log_channel)
            .await
            .unwrap();

        // Call Reset
        handle.reset().await.unwrap();
        assert!(
            m.reset_called.load(Ordering::SeqCst),
            "Reset function should have been called"
        );

        // Cleanup happens when `handle` is dropped at end of scope
    }

    #[tokio::test]
    async fn test_foreign_class_register() {
        let m = mocks();

        let export = ClassExport {
            name: NAME.as_ptr(),
            name_len: NAME.len(),
            ptr: ptr::null(),
            len: 0,
            init: m.init,
            reset: ClassResetFn::Null,
            register: m.register,
        };

        let class = Arc::new(
            unsafe { ForeignClass::from_export_inter("test".to_string(), None, export) }.unwrap(),
        );
        let log_channel = create_log(0, 0, 100);

        let config = PyroVec::ok();
        let handle = class
            .create_instance(config.py_ref(), 0, log_channel)
            .await
            .unwrap();

        let client_data = PyroVec::ok();
        let result = handle.register(client_data.py_ref()).await.unwrap();

        assert!(m.register_called.load(Ordering::SeqCst));
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_foreign_class_register_null() {
        let m = mocks();

        let export = ClassExport {
            name: NAME.as_ptr(),
            name_len: NAME.len(),
            ptr: ptr::null(),
            len: 0,
            init: m.init,
            reset: ClassResetFn::Null,
            register: ClientRegisterFn::Null, // Explicitly testing null path
        };
        let log_channel = create_log(0, 0, 100);

        let class = Arc::new(
            unsafe { ForeignClass::from_export_inter("test".to_string(), None, export) }.unwrap(),
        );
        let config = PyroVec::ok();
        let handle = class
            .create_instance(config.py_ref(), 0, log_channel)
            .await
            .unwrap();

        let client_data = PyroVec::ok();
        let result = handle.register(client_data.py_ref()).await;

        assert!(result.is_ok());
    }
}
