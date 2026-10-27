//! Tracing ids for the header: 128-bit trace ids and 64-bit span ids, the
//! W3C / OpenTelemetry sizes. Both are unique without coordination and cost
//! about a nanosecond each.
//!
//! These are the fallback for when no OpenTelemetry context is active. When
//! one is, its ids go in the header and nothing here is called.
//!
//! # Trace ids ([`next_trace_id`], `u128`)
//!
//! ```text
//!   127                              32 31             0
//!  ┌──────────────────────────────────┬────────────────┐
//!  │ prefix: 96 random bits           │ counter: u32   │   raw
//!  └──────────────────────────────────┴────────────────┘
//!  then the low 64 bits go through mix64
//! ```
//!
//! Each thread draws a random 96-bit prefix the first time it asks for a
//! trace id, then counts up from 1 in the low 32 bits. When the counter runs
//! out, the thread draws a fresh prefix and starts again. A *generator* is one
//! (prefix, counter run) pair; every generator owns a disjoint block of
//! 2³² − 1 ids unless two generators draw the same prefix.
//!
//! The low 64 bits of that raw id then pass through `mix64`, a bijective
//! mixer (the splitmix64 finalizer). OpenTelemetry's `TraceIdRatioBased`, and
//! the W3C Trace Context Level 2 "random" flag, treat the low half of a trace
//! id as the random number to sample on. Raw ids keep the thread's fixed
//! prefix there, so one thread would sample all of its traces or none. After
//! the mix every id's low 64 bits look uniformly random.
//!
//! The mix is a bijection and the top 64 bits pass through untouched, so two
//! distinct raw ids still give two distinct trace ids: uniqueness and the
//! collision table below are unchanged. It maps only `0` to `0`, and the raw
//! low half is never `0` (the counter starts at 1), so a trace id is never the
//! W3C-invalid all-zero id.
//!
//! # Span ids ([`next_span`], `u64`)
//!
//! ```text
//!  span = mix64(key + n)      key: random u64 per thread, n = 1, 2, 3, ...
//! ```
//!
//! OpenTelemetry only needs a span id to be unique *within its trace*. Each
//! thread draws a random 64-bit key once and walks `key + 1, key + 2, ...`
//! through the same bijective mixer. Within a thread ids never repeat (the
//! counter would take 2⁶⁴ spans to wrap). Two threads collide only if their
//! walks overlap, which for runs of `N₁` and `N₂` spans has probability about
//! `(N₁ + N₂) / 2⁶⁴`: no worse than drawing every span id at random, which is
//! what the OpenTelemetry SDK does. `0` is W3C-invalid, so the one input that
//! mixes to `0` is skipped.
//!
//! # Costs
//!
//! **Hot path**, either function: one thread-local load, one add, one
//! predictable branch, the mixer (two multiplies, three xor-shifts on a
//! register), one thread-local store. No syscall, no atomic, no clock read,
//! no allocation, no cross-core traffic, so it scales linearly with threads.
//! Roughly 1 ns.
//!
//! **Reseed**: one draw from std's [`RandomState`] (OS entropy the first time
//! on each thread, a key bump after that) plus one or two SipHash-1-3 rounds.
//! Paid on a thread's first id of each kind; after that, trace ids reseed
//! once every 2³² − 1 ids (every ~86 s at 50 M/s on one thread) and span ids
//! never. Amortised, it rounds to zero.
//!
//! **Memory**: 48 bytes of thread-local state per thread that has made ids
//! (trace prefix `u128` + counter `u32`, padded; span key `u64` + counter
//! `u64`). Nothing global.
//!
//! **Wire**: the header's 16-byte `trace_id` at 16..32 (one aligned 16-byte
//! store) and 8-byte `parent_span` at 32..40. Nothing is spent on embedded
//! time: the header already carries a timestamp at 40..48.
//!
//! **Trace id collision risk**: two generators collide only if they draw the
//! same 96-bit prefix. Over `N` generators in the whole cluster's lifetime,
//! counting every thread on every machine, every restart, and every reseed:
//!
//! | `N` (generators, ever) | P(any collision) ≈ N² / 2⁹⁷ |
//! | --- | --- |
//! | 2³⁰ (10k machines × 100 threads × 1k restarts) | 2⁻³⁷ |
//! | 2⁴⁰ | 2⁻¹⁷ |
//! | 2⁴⁸ | 2⁻¹ |
//!
//! That is comparable to UUIDv4 (122 random bits per *id*) because here the
//! random bits are paid per *generator* and the counter partitions the rest.
//!
//! # What this is not
//!
//! - **Not secret.** The mixer is a fixed public function over a counter, so
//!   anyone who sees a few ids can predict the next. Fine for tracing; don't
//!   use them as tokens or nonces.
//! - **Not fork-safe.** A `fork()` without `exec` copies the thread-local
//!   state, so parent and child hand out the same ids until one reseeds.
//!   Restoring a VM from a memory snapshot taken after threads seeded has the
//!   same effect. Call [`reseed_span`] in the child / after restore.
//! - **Not ordered.** The mixer scrambles issue order; order comes from the
//!   header timestamp.

use std::cell::Cell;
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};

/// Bits of the trace id's raw counter. The prefix gets the remaining
/// `128 - COUNTER_BITS`.
pub const COUNTER_BITS: u32 = 32;

/// Mask of the counter half; the prefix is everything outside it.
const COUNTER_MASK: u128 = (1u128 << COUNTER_BITS) - 1;

thread_local! {
    /// `(prefix, last counter)`. A counter of `u32::MAX` doubles as
    /// "unseeded", so the first call and counter exhaustion share the one
    /// cold branch.
    static TRACE: Cell<(u128, u32)> = const { Cell::new((0, u32::MAX)) };

    /// `(key, last counter)`. A counter of `0` means "unseeded"; once seeded
    /// it only grows.
    static SPAN: Cell<(u64, u64)> = const { Cell::new((0, 0)) };
}

/// A fresh W3C trace id: unique across the cluster, with uniformly mixed low
/// 64 bits so ratio samplers see a random number. Never `0`.
#[inline]
pub fn next_trace_id() -> u128 {
    let raw = next_raw_trace();
    let hi = raw & !(u64::MAX as u128);
    hi | mix64(raw as u64) as u128
}

/// A fresh span id, for the header's `parent_span` when no OpenTelemetry
/// span is active. Unique within any trace. Never `0`.
#[inline]
pub fn next_span() -> u64 {
    SPAN.with(|s| {
        let (mut key, mut n) = s.get();
        if n == 0 {
            key = draw_random() as u64;
        }
        loop {
            n += 1;
            let id = mix64(key.wrapping_add(n));
            // Only `key + n == 0` mixes to `0`: once per 2⁶⁴ ids, if ever.
            if id != 0 {
                s.set((key, n));
                return id;
            }
        }
    })
}

/// Throw away this thread's trace prefix and span key; the next
/// [`next_trace_id`] / [`next_span`] draws new ones. Call after `fork()` in
/// the child, or after restoring a memory snapshot.
pub fn reseed_span() {
    TRACE.with(|s| s.set((0, u32::MAX)));
    SPAN.with(|s| s.set((0, 0)));
}

/// The unmixed 96/32 trace id: prefix in the top 96 bits, counter (from 1)
/// in the low 32.
#[inline]
fn next_raw_trace() -> u128 {
    TRACE.with(|s| {
        let (mut prefix, mut n) = s.get();
        if n == u32::MAX {
            prefix = draw_random() & !COUNTER_MASK;
            n = 0;
        }
        n += 1;
        s.set((prefix, n));
        prefix | n as u128
    })
}

/// The splitmix64 finalizer. A bijection on `u64` (each step is invertible:
/// xor-shift right, or multiply by an odd constant) that fixes `0`.
#[inline(always)]
fn mix64(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// 128 random bits.
///
/// `RandomState::new()` is keyed from OS entropy on a thread's first call and
/// bumps a key on each later one, so every call here keys a fresh SipHash
/// PRF. The pid is mixed in so a forked child that calls [`reseed_span`]
/// diverges from its parent even though it inherited the same keys.
#[cold]
fn draw_random() -> u128 {
    let state = RandomState::new();
    let word = |i: u64| {
        let mut h = state.build_hasher();
        h.write_u64(i);
        h.write_u32(std::process::id());
        h.finish()
    };
    ((word(0) as u128) << 64) | word(1) as u128
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn raw_trace_is_sequential_within_a_thread() {
        reseed_span();
        let a = next_raw_trace();
        let b = next_raw_trace();
        assert_eq!(a & COUNTER_MASK, 1);
        assert_eq!(b, a + 1);
        assert_eq!(a & !COUNTER_MASK, b & !COUNTER_MASK);
    }

    #[test]
    fn reseed_changes_trace_prefix() {
        let a = next_raw_trace();
        reseed_span();
        let b = next_raw_trace();
        assert_ne!(a & !COUNTER_MASK, b & !COUNTER_MASK);
        assert_eq!(b & COUNTER_MASK, 1);
    }

    #[test]
    fn trace_counter_exhaustion_reseeds() {
        TRACE.with(|s| s.set((0xabcd << COUNTER_BITS, u32::MAX - 1)));
        let last = next_raw_trace();
        assert_eq!(last, (0xabcd << COUNTER_BITS) | u32::MAX as u128);
        let next = next_raw_trace();
        assert_ne!(next & !COUNTER_MASK, 0xabcd << COUNTER_BITS);
        assert_eq!(next & COUNTER_MASK, 1);
    }

    #[test]
    fn trace_ids_keep_the_high_half_and_are_unique() {
        reseed_span();
        let ids: Vec<u128> = (0..10_000).map(|_| next_trace_id()).collect();
        let distinct: HashSet<u128> = ids.iter().copied().collect();
        assert_eq!(distinct.len(), ids.len());
        assert!(ids.iter().all(|&id| id != 0 && id as u64 != 0));
        // Same generator, so the untouched top 64 bits agree.
        assert!(ids.iter().all(|&id| id >> 64 == ids[0] >> 64));
    }

    #[test]
    fn trace_ids_sample_evenly_within_one_thread() {
        // What TraceIdRatioBased looks at: the low 64 bits, top bits first.
        // A 10% sampler on raw ids would take all or none of one thread's
        // traces; on mixed ids it takes close to 10%.
        reseed_span();
        let threshold = u64::MAX / 10;
        let n = 100_000;
        let taken = (0..n)
            .filter(|_| (next_trace_id() as u64) < threshold)
            .count();
        assert!((8_000..12_000).contains(&taken), "took {taken} of {n}");
    }

    #[test]
    fn span_ids_are_unique_and_nonzero() {
        reseed_span();
        let ids: HashSet<u64> = (0..10_000).map(|_| next_span()).collect();
        assert_eq!(ids.len(), 10_000);
        assert!(!ids.contains(&0));
    }

    #[test]
    fn span_skips_the_input_that_mixes_to_zero() {
        // Seeded, last n = 1: the next candidate is key + 2 == 0, which
        // mixes to 0 and must be skipped in favour of key + 3 == 1.
        SPAN.with(|s| s.set((u64::MAX - 1, 1)));
        let id = next_span();
        assert_ne!(id, 0);
        assert_eq!(id, mix64(1));
        assert_eq!(SPAN.with(|s| s.get()), (u64::MAX - 1, 3));
    }

    #[test]
    fn reseed_changes_span_key() {
        next_span();
        let (a, _) = SPAN.with(|s| s.get());
        reseed_span();
        next_span();
        let (b, n) = SPAN.with(|s| s.get());
        assert_ne!(a, b);
        assert_eq!(n, 1);
    }

    #[test]
    fn mix64_fixes_zero_and_moves_everything_else() {
        assert_eq!(mix64(0), 0);
        assert_ne!(mix64(1), 0);
        assert_ne!(mix64(1), 1);
    }

    #[test]
    fn threads_get_distinct_seeds() {
        let seeds: HashSet<(u128, u64)> = (0..64)
            .map(|_| {
                std::thread::spawn(|| {
                    next_span();
                    (next_raw_trace() & !COUNTER_MASK, SPAN.with(|s| s.get().0))
                })
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|h| h.join().unwrap())
            .collect();
        assert_eq!(seeds.len(), 64);
    }
}
