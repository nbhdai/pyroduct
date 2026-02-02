//! Test module 2: Uses test_cap2 transform capability
//!
//! Module that transforms input strings using the async transform capability.

use test_cap2::TransformClient;

#[pyroduct::module(output = (original, transformed, transform_count))]
pub fn call(input: &str) -> Result<(String, String, usize), String> {
    let client = TransformClient {
        prefix: "[TEST] ".to_string(),
    };
    client.register()?;

    let original = input.to_string();
    let transformed = client.transform(input.to_string())?;
    let transform_count = client.get_transform_count()?;

    Ok((original, transformed, transform_count))
}