//! Shared test support. Each integration-test binary that needs the allocation
//! counter declares it as its global allocator:
//!
//! ```ignore
//! mod support;
//! #[global_allocator]
//! static ALLOC: support::alloc_counter::CountingAlloc =
//!     support::alloc_counter::CountingAlloc::new();
//! ```
#![allow(dead_code, unused_imports)]

pub mod alloc_counter;
pub mod bits;
pub mod read_back;
