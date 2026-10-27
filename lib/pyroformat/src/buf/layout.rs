//! Allocation geometry.
//!
//! One file so the numbers that define the layout — the prefix size, the field
//! offsets, the exact allocation — have a single home. Every one of them is
//! pinned by a test; the prefix and the header must not
//! quietly re-grow slack.

use std::alloc::Layout;

/// Local bookkeeping ahead of the message pointer. Never transmitted.
pub const PREFIX_SIZE: usize = 32;
pub use super::header::{ALIGN, HEADER_SIZE, PROTOCOL_VERSION};

// Prefix field offsets, *negative* from the message pointer.
pub(crate) const OFF_CTX: usize = 32;
pub(crate) const OFF_VTABLE: usize = 24;
pub(crate) const OFF_REFCOUNT: usize = 16;
pub(crate) const OFF_CAPACITY: usize = 12;
pub(crate) const OFF_POOL_CLASS: usize = 8;
pub(crate) const OFF_LOCAL_FLAGS: usize = 6;
pub(crate) const OFF_GENERATION: usize = 4;

/// `local_flags` bits.
/// The buffer has been sealed: header stamped, capacity frozen, refcount live.
pub const FLAG_SEALED: u16 = 1 << 0;
/// The buffer belongs to a pool and returns to its free list at refcount zero.
pub const FLAG_POOL_OWNED: u16 = 1 << 1;

/// `domain_id` claims the upper 12 of the 14 spare bits in `local_flags`
/// ([docs/io-plan.md §4.1](../../docs/io-plan.md)), leaving 2 bits still spare.
/// A buffer released into the wrong domain is then a debug assertion rather
/// than heap corruption.
pub const DOMAIN_ID_SHIFT: u16 = 4;
pub const DOMAIN_ID_BITS: u16 = 12;
pub const DOMAIN_ID_MASK: u16 = ((1u16 << DOMAIN_ID_BITS) - 1) << DOMAIN_ID_SHIFT;

/// Pack a `domain_id` into a `local_flags` value, preserving every other bit.
/// Used by `Domain::hand_out` (the `io` feature); `#[allow(dead_code)]`
/// keeps a default (`io`-less) build warning-free.
#[inline]
#[allow(dead_code)]
pub(crate) fn pack_domain_id(flags: u16, domain_id: u16) -> u16 {
    (flags & !DOMAIN_ID_MASK) | ((domain_id << DOMAIN_ID_SHIFT) & DOMAIN_ID_MASK)
}

/// Extract the `domain_id` packed into a `local_flags` value.
#[inline]
pub(crate) fn unpack_domain_id(flags: u16) -> u16 {
    (flags & DOMAIN_ID_MASK) >> DOMAIN_ID_SHIFT
}

/// `capacity: u32` caps a message at 4 GiB, matching the header's `u32` length.
/// A hard limit, reported as [`PyroError::MessageTooLarge`], never truncated.
pub const MAX_PAYLOAD: usize = u32::MAX as usize;

/// The exact allocation for `payload_cap` payload bytes: prefix + header +
/// payload, and nothing else.
///
/// Not padded to alignment, so the allocation size is *exactly*
/// `PREFIX_SIZE + HEADER_SIZE + payload_cap`. This is fix (1) of §2: the
/// original double-counted the header, because `PyroInner` already contained
/// `header: [u8; 16]` and `with_capacity` added `HEADER_SIZE` on top.
#[inline]
pub(crate) fn layout_for(payload_cap: usize) -> Layout {
    debug_assert!(payload_cap <= MAX_PAYLOAD);
    // Safe: ALIGN is a power of two, and the size is bounded by
    // 48 + u32::MAX, far below isize::MAX on every supported target.
    Layout::from_size_align(PREFIX_SIZE + HEADER_SIZE + payload_cap, ALIGN)
        .expect("buffer layout is always valid")
}

/// Round a payload capacity up to a multiple of [`ALIGN`], so the total
/// allocation stays a multiple of 16 across reallocations.
#[inline]
pub(crate) fn round_up(cap: usize) -> usize {
    (cap + ALIGN - 1) & !(ALIGN - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefix_offsets_tile_the_prefix() {
        // Fully packed: the five bookkeeping fields plus ctx and vtable exactly
        // fill 32 bytes, with no padding byte anywhere.
        assert_eq!(OFF_CTX, PREFIX_SIZE);
        assert_eq!(OFF_CTX - OFF_VTABLE, 8); // ctx: *mut c_void
        assert_eq!(OFF_VTABLE - OFF_REFCOUNT, 8); // vtable: *const _
        assert_eq!(OFF_REFCOUNT - OFF_CAPACITY, 4); // ref_count: u32
        assert_eq!(OFF_CAPACITY - OFF_POOL_CLASS, 4); // capacity: u32
        assert_eq!(OFF_POOL_CLASS - OFF_LOCAL_FLAGS, 2); // pool_class: u16
        assert_eq!(OFF_LOCAL_FLAGS - OFF_GENERATION, 2); // local_flags: u16
        assert_eq!(OFF_GENERATION, 4); // generation: u32, ending at the header
        // Both regions are multiples of 16, so payload alignment survives.
        assert!(PREFIX_SIZE.is_multiple_of(ALIGN));
        assert!(HEADER_SIZE.is_multiple_of(ALIGN));
    }

    #[test]
    fn domain_id_round_trips_without_disturbing_other_bits() {
        for domain_id in [0u16, 1, 42, 0x0FFF] {
            for base in [
                0u16,
                FLAG_SEALED,
                FLAG_POOL_OWNED,
                FLAG_SEALED | FLAG_POOL_OWNED,
            ] {
                let packed = pack_domain_id(base, domain_id);
                assert_eq!(unpack_domain_id(packed), domain_id);
                assert_eq!(packed & !DOMAIN_ID_MASK, base);
            }
        }
    }

    #[test]
    fn domain_id_mask_leaves_two_bits_spare() {
        assert_eq!(DOMAIN_ID_MASK.count_ones(), 12);
        let used = FLAG_SEALED | FLAG_POOL_OWNED | DOMAIN_ID_MASK;
        assert_eq!((!used).count_ones(), 2);
    }

    #[test]
    fn allocation_is_exactly_prefix_header_payload() {
        for cap in [0usize, 1, 15, 16, 17, 4096] {
            let l = layout_for(cap);
            assert_eq!(l.size(), PREFIX_SIZE + HEADER_SIZE + cap, "cap {cap}");
            assert_eq!(l.align(), ALIGN);
        }
    }
}
