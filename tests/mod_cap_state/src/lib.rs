//! Test module 1: Uses test_cap1 counter capability
//!
//! Simple module that increments a counter and returns the result.

use cap_state::CounterClient;

#[pyroduct::module(output = (count, incremented))]
pub fn call(input: &str) -> Result<(u64, u64), String> {
    let start: u64 = input.parse().map_err(|e| format!("Parse error: {}", e))?;

    let client = CounterClient { start_value: start };
    client.register()?;

    let count = client.get_count()?;
    let incremented = client.increment()?;

    Ok((count, incremented))
}