//! Test module 1: Uses test_cap1 counter capability
//!
//! Simple module that increments a counter and returns the result.

use pyroduct::Capture;
use state::{CounterClient, CounterClientMethods};

#[pyroduct::module(output = (count, incremented))]
pub fn call(input: &str) -> Result<(u64, u64)> {
    let start: u64 = input.parse().capture("We expect a u64")?;

    let client = CounterClient { start_value: start }.register()?;

    let count = client.get_count()?;
    let incremented = client.increment()?;

    Ok((count, incremented))
}
