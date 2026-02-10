use pyro_vec::library;

// Test: Basic library registration
library!("meta");

fn main() {
    // Verify the Library struct was generated
    assert_eq!(Library::NAME, env!("CARGO_PKG_NAME"));
}