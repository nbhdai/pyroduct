use bridge_vec::{library, captured::CapturedError};

library!("pyroduct-meta-v1");

#[test]
fn test_library_identity() {
    // Verify the generated constants match the package info
    assert_eq!(Library::META, "pyroduct-meta-v1");
    assert!(!Library::AUTHORS.is_empty());
}

#[test]
fn test_captured_error_includes_library_ident() {
    Library::register_info();
    // 2. Create a new captured error
    // The constructor calls APP_IDENTITY.get() internally
    let error = CapturedError::new("Something went sideways");

    // 3. Verify the library info is present
    let lib_info = error.library.as_ref().expect("Library info should be captured");
    println!("{lib_info:?}");

    // Check that the metadata we passed to the macro is there
    assert_eq!(lib_info.meta, "pyroduct-meta-v1");

    // Check that Cargo environment variables were correctly pulled
    // These will match the [package] section of your bridge_vec or test crate
    assert_eq!(lib_info.name, env!("CARGO_PKG_NAME"));
    assert_eq!(lib_info.version, env!("CARGO_PKG_VERSION"));
    
    // 4. Verify Display implementation includes the message
    let display_msg = format!("{}", error);
    assert!(display_msg.contains("Something went sideways"));
}

#[test]
fn test_error_serialization_preserves_library_info() {
    Library::register_info();
    let error = CapturedError::new("Persistence failure");
    
    // Serialize to JSON (as used in bridge_vec internal encoding)
    let serialized = serde_json::to_string_pretty(&error).expect("Should serialize to JSON");
    println!("{serialized}");
    // Ensure the library identity is present in the serialized output
    assert!(serialized.contains("pyroduct-meta-v1"));
    assert!(serialized.contains(env!("CARGO_PKG_NAME")));
}