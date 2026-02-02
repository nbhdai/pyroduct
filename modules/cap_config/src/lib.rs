//! The behavior of this module changes based on the configuration 
//! of the linked capability

use config::{TransformClient, TransformClientMethods};

#[pyroduct::module(output = (original, transformed, transform_count))]
pub fn call(input: &str) -> Result<(String, String, u64), String> {
    let client = TransformClient {
        prefix: "[TEST] ".to_string(),
    }.register()?;

    let original = input.to_string();
    let transformed = client.transform(input.to_string())?;
    let transform_count = client.get_transform_count()? as u64;

    Ok((original, transformed, transform_count))
}