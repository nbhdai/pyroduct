//! Bool-run builders for the bool tests' packed and wide-array variants.

use pyroformat::BitVec;

/// A bool run as a packed `pyroformat::BitVec` (`BitVec<u8, Lsb0>`), the
/// alternative to a `Vec<bool>` that must match the same fixture.
pub fn bits(values: &[bool]) -> BitVec {
    values.iter().copied().collect()
}

/// A `[bool; N]` holding `value`'s low `N` bits, least significant first —
/// so the array packs to `value`'s little-endian bytes, which is how the
/// `_BOOL_8`/`_BOOL_16` fixtures spell their payloads.
pub fn lsb_bools<const N: usize>(value: u16) -> [bool; N] {
    std::array::from_fn(|i| (value >> i) & 1 == 1)
}
