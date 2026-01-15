//! CPU Info Capability
//! 
//! Pattern: Stateless (no state on either side)

use capability_derive::*;

// ============================================================================
// CAPABILITY DEFINITION
// ============================================================================

/// A simple stateless capability for CPU information.
#[capability_function]
pub fn get_cpu_count() -> u32 {
    std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(1)
}

#[capability_function]
pub fn get_architecture() -> u32 {
    std::env::consts::ARCH.to_string()
}

capability_export!(env = "cpu_info", functions = [get_cpu_count, get_architecture]);