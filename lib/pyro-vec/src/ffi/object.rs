use std::fmt;
use std::ptr::NonNull;
use crate::vec_buf::{PyroVec, PyroVecPtr};

// ============================================================================
// Function pointer types
// ============================================================================

/// Function pointer to drop the opaque object.
pub type PyroObjectDropper = unsafe extern "C" fn(ptr: *mut u8);

/// Function pointer to the constructor in the foreign library.
///
/// This is the symbol you would look up in the dylib (e.g., "create_my_object").
pub type PyroObjectConstructor = unsafe extern "C" fn(config: PyroVecPtr) -> PyroObjectPtr;

// ============================================================================
// PyroForeignClass — The Factory
// ============================================================================

/// Represents a "Class" loaded from a dynamic library.
///
/// It holds the constructor function pointer and allows you to instantiate
/// new `PyroObject`s safely.
#[derive(Clone, Copy)]
pub struct PyroForeignClass {
    constructor: PyroObjectConstructor,
}

impl PyroForeignClass {
    /// Creates a new class definition from a raw constructor function pointer.
    ///
    /// # Safety
    /// The caller must ensure the function pointer is valid and matches the
    /// `PyroObjectConstructor` signature.
    pub unsafe fn new(constructor: PyroObjectConstructor) -> Self {
        Self { constructor }
    }

    /// Instantiates a new object from this class with the given configuration.
    ///
    /// This consumes the `config` vector, passes it to the foreign library,
    /// and returns a safe, owned `PyroObject`.
    pub fn instantiate(&self, config: PyroVec) -> Option<PyroObject> {
        // 1. Convert safe config to raw FFI pointer
        let raw_config = config.into_raw();

        // 2. Call the foreign constructor
        // SAFETY: We trust the constructor pointer provided in `new`.
        let raw_obj = unsafe { (self.constructor)(raw_config) };

        // 3. Convert raw FFI object back to safe wrapper
        // SAFETY: We assume the foreign constructor returns a valid PyroObjectPtr
        unsafe { PyroObject::from_raw(raw_obj) }
    }
}

impl fmt::Debug for PyroForeignClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PyroForeignClass")
            .field("constructor_ptr", &(self.constructor as usize))
            .finish()
    }
}

// ============================================================================
// PyroObjectPtr — The FFI-safe representation
// ============================================================================

#[repr(C)]
pub struct PyroObjectPtr {
    pub ptr: *mut u8,
    pub dropper: PyroObjectDropper,
}

// ============================================================================
// PyroObject — The Safe Instance
// ============================================================================

pub struct PyroObject {
    ptr: NonNull<u8>,
    dropper: PyroObjectDropper,
}

// SAFETY: We assume the opaque object is Send/Sync.
unsafe impl Send for PyroObject {}
unsafe impl Sync for PyroObject {}

impl PyroObject {
    pub unsafe fn from_raw(raw: PyroObjectPtr) -> Option<Self> {
        let ptr = NonNull::new(raw.ptr)?;
        Some(Self {
            ptr,
            dropper: raw.dropper,
        })
    }

    pub fn into_raw(self) -> PyroObjectPtr {
        let ptr = self.ptr.as_ptr();
        let dropper = self.dropper;
        std::mem::forget(self);
        PyroObjectPtr { ptr, dropper }
    }

    pub fn as_ptr(&self) -> *mut u8 {
        self.ptr.as_ptr()
    }
}

impl Drop for PyroObject {
    fn drop(&mut self) {
        unsafe {
            (self.dropper)(self.ptr.as_ptr());
        }
    }
}

impl fmt::Debug for PyroObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PyroObject")
            .field("ptr", &self.ptr)
            .field("dropper", &(self.dropper as usize))
            .finish()
    }
}