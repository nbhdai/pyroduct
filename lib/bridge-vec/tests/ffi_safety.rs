use bridge_vec::{
    BridgeVec, DataStatus, bridgeable, Bridgeable, ffi::{RkyvFfiError, access_from_ffi}
};
use rkyv::{Archive, Deserialize};

// --- Test Structures ---

#[bridgeable(derive(Debug, PartialEq))]
#[derive(Debug, PartialEq)]
struct UserData {
    id: u32,
    payload: String,
}

#[bridgeable(derive(Debug, PartialEq))]
#[derive(Debug, PartialEq)]
struct UserError {
    code: u16,
    msg: String,
}

#[derive(Archive, Deserialize, Debug)]
struct PanicOnSerialize;

// Implement a crashing serializer
impl<S: rkyv::rancor::Fallible + ?Sized> rkyv::Serialize<S> for PanicOnSerialize {
    fn serialize(&self, _serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        panic!("Intentional panic during serialization test");
    }
}

impl Bridgeable for PanicOnSerialize {
    fn serialize(&self) -> Result<BridgeVec, rkyv::rancor::Error> {
        ::bridge_vec::BridgeVec::serialize_from(self)
    }

    fn unchecked_parse(vec: BridgeVec) -> Result<bridge_vec::TypedBuf<Self>, rkyv::rancor::Error> {
        vec.unchecked_parse::<Self>()
    }
}

// --- Tests ---

#[test]
fn test_ffi_success_path() {
    let data = UserData { id: 101, payload: "Success".to_string() };
    let result: Result<&UserData, &UserError> = Ok(&data);

    let vec = BridgeVec::serialize_result(result);
    
    // Verify Header Status
    assert_eq!(vec.status(), DataStatus::ValidData as u16);
    println!("Vec {:?}", vec);

    // Verify Access
    let access = unsafe { access_from_ffi::<UserData, UserError>(vec.as_ptr()) };
    println!("{:?}", access);
    match access {
        Ok(Ok(archived)) => {
            assert_eq!(archived.id, 101);
            assert_eq!(archived.payload, "Success");
        },
        _ => panic!("Expected Ok(Ok(..))"),
    }
}

#[test]
fn test_ffi_user_error_path() {
    let err = UserError { code: 500, msg: "Server exploded".to_string() };
    let result: Result<&UserData, &UserError> = Err(&err);

    let vec = BridgeVec::serialize_result(result);
    
    assert_eq!(vec.status(), DataStatus::UserError as u16);

    let access = unsafe { access_from_ffi::<UserData, UserError>(vec.as_ptr()) };
    match access {
        Ok(Err(archived_err)) => {
            assert_eq!(archived_err.code, 500);
            assert_eq!(archived_err.msg, "Server exploded");
        },
        _ => panic!("Expected Ok(Err(..))"),
    }
}

#[test]
fn test_ffi_panic_safety_catch() {
    // Attempt to serialize a struct that panics
    let p = PanicOnSerialize;
    
    // We cast it to match the expected generic signature
    let result: Result<&PanicOnSerialize, &PanicOnSerialize> = Ok(&p);
    
    // This MUST NOT abort the process. It must return a buffer with RkyvFfiError status.
    let vec = BridgeVec::serialize_result(result);
    
    assert_eq!(vec.status(), DataStatus::TransportError as u16);
    
    // Decode the error
    let access = unsafe { access_from_ffi::<UserData, UserError>(vec.as_ptr()) };
    
    match access {
        Err(RkyvFfiError::RemoteSerializationPanic(msg)) => {
            assert!(msg.contains("Intentional panic"), "Error message should contain panic payload");
        },
        _ => panic!("Expected RemoteSerializationPanic, got {:?}", access),
    }
}

#[test]
fn test_validation_security() {
    // 1. Serialize a u64 (8 bytes)
    let wrong_data = 999999u64;
    let mut vec = BridgeVec::serialize_from(&wrong_data).unwrap();
    
    // 2. Maliciously mark it as ValidData for UserData struct
    vec.set_status(DataStatus::ValidData as u16); 

    // 3. Attempt to access it as UserData. 
    let access = unsafe { access_from_ffi::<UserData, UserError>(vec.as_ptr()) };
    
    match access {
        Err(RkyvFfiError::ValidationFailed(_)) => {}, // Success: it caught the lie
        Ok(_) => panic!("Validation passed on invalid data! Security breach."),
        Err(e) => panic!("Wrong error type returned: {:?}", e),
    }
}

#[test]
fn test_null_ptr_safety() {
    let result = unsafe { access_from_ffi::<UserData, UserError>(std::ptr::null()) };
    assert!(matches!(result, Err(RkyvFfiError::NullPointer)));
}