//! Step V0: the allocation counter's own tests. Nothing downstream can *prove*
//! allocation-freedom until this is known to work, so it is tested first.

mod support;

use support::alloc_counter::{AllocGuard, CountingAlloc};

#[global_allocator]
static ALLOC: CountingAlloc = CountingAlloc::new();

#[test]
fn counts_a_single_allocation() {
    let g = AllocGuard::arm();
    let v: Vec<u8> = Vec::with_capacity(64);
    assert_eq!(g.allocs(), 1);
    assert!(g.bytes() >= 64);
    drop(v);
    assert_eq!(g.deallocs(), 1);
}

#[test]
fn counts_nothing_while_disarmed() {
    let g = AllocGuard::arm();
    g.disarm();
    let _v: Vec<u8> = Vec::with_capacity(64);
    assert_eq!(g.allocs(), 0);
}

#[test]
fn a_zero_capacity_vec_does_not_allocate() {
    let g = AllocGuard::arm();
    let v: Vec<u8> = Vec::new();
    assert_eq!(g.allocs(), 0);
    drop(v);
}

#[test]
fn counts_a_reallocation_as_an_allocation_event() {
    let mut v: Vec<u8> = Vec::with_capacity(1);
    let g = AllocGuard::arm();
    // Forces a grow: this is the event a "no reallocation" assertion watches.
    v.extend_from_slice(&[0u8; 256]);
    assert!(g.allocs() >= 1);
}

#[test]
fn counters_reset_on_each_arm() {
    {
        let g = AllocGuard::arm();
        let _v: Vec<u8> = Vec::with_capacity(8);
        assert_eq!(g.allocs(), 1);
    }
    let g = AllocGuard::arm();
    assert_eq!(g.allocs(), 0);
    assert_eq!(g.deallocs(), 0);
    assert_eq!(g.bytes(), 0);
}

#[test]
fn counting_is_per_thread() {
    let g = AllocGuard::arm();
    // Another thread's allocations must not land in this thread's counters,
    // otherwise every assertion here would be at the mercy of the harness.
    std::thread::spawn(|| {
        let _v: Vec<u8> = Vec::with_capacity(4096);
    })
    .join()
    .unwrap();
    // Joining a thread does allocate on this thread in some std versions, so
    // this asserts the *foreign* 4096-byte allocation did not appear.
    assert!(
        g.bytes() < 4096,
        "foreign allocation leaked into our counts"
    );
}
