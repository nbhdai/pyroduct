use crate::StructureWord;

use super::kind::{KindMeta, WireKind};

pub const INODE_SIZE: usize = 12;

/// Error returned by [`Inode::decode`]: the input was too short, or the
/// leading kind byte didn't match any [`WireKind`] discriminant.
///
/// Deliberately minimal — `pyro_schema` has no dependency on `pyroformat`'s
/// own `WireError`, so callers (e.g. `pyroformat::spec::PyroSchema::decode`)
/// that need a richer error map this away themselves (they already did,
/// via `map_err`, even before this crate split).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum InodeDecodeError {
    #[error("inode buffer too short")]
    TooShort,
    #[error("invalid inode kind discriminant: {0}")]
    InvalidKind(u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Inode {
    pub kind: WireKind,
    pub kind_meta: KindMeta,
}

impl Inode {
    pub fn encode(&self) -> [u8; INODE_SIZE] {
        let mut buf = [0u8; INODE_SIZE];
        buf[0] = self.kind.encode();
        buf[1..12].copy_from_slice(&self.kind_meta.encode());
        buf
    }

    pub fn decode(bytes: &'_ [u8]) -> Result<Self, InodeDecodeError> {
        if bytes.len() < INODE_SIZE {
            return Err(InodeDecodeError::TooShort);
        }
        let raw_kind = bytes[0];
        let kind = WireKind::parse(raw_kind & 0x7F).map_err(InodeDecodeError::InvalidKind)?;
        let mut meta_bytes = [0u8; 11];
        meta_bytes.copy_from_slice(&bytes[1..12]);
        let kind_meta = KindMeta::decode(kind, meta_bytes);

        Ok(Self { kind, kind_meta })
    }

    /// Alignment / bucket key for this inode: the byte width of a single
    /// element of the underlying wire kind (1, 2, 4, 8, or 16; 0 for
    /// Group/Map, which never land in the fixed-item buckets).
    pub const fn alignment(&self) -> usize {
        self.kind.byte_size()
    }

    /// Number of bytes this inode reserves in the fixed region.
    ///
    /// - nesting == 0: `alignment() * max(fixed_size, 1)` — a plain scalar
    ///   occupies one element inline; a nullable fixed array
    ///   (`Option<[T; N]>`, which uses nesting=0 by convention) occupies N
    ///   elements inline.
    /// - nesting >= 1: always 8 bytes — a `[start_offset: u32, count: u32]`
    ///   pointer pair into the dynamic region, regardless of the element
    ///   type or fixed length (`Vec<T>`, `[T; N]`, `Str`, `Bytes` all store
    ///   their payload out-of-line; see `integration/writer/fixed.rs` and
    ///   `wire/writers/fixed_list.rs`).
    pub const fn size<W: StructureWord>(&self) -> usize {
        if self.nesting() >= 1 && (self.is_nullable_fixed() || self.is_nullable_group()) {
            3 * W::SIZE
        } else if self.nesting() >= 1 || matches!(self.kind, WireKind::Str | WireKind::Bytes) {
            2 * W::SIZE
        } else {
            let base = self.alignment();
            let fs = self.fixed_size() as usize;
            if fs > 0 { base * fs } else { base }
        }
    }

    pub const fn fixed_size(&self) -> u16 {
        self.kind_meta.fixed_size()
    }

    pub const fn nesting(&self) -> u8 {
        self.kind_meta.nesting()
    }

    pub const fn topology_offset(&self) -> u16 {
        self.kind_meta.topology_offset()
    }

    pub const fn data_offset(&self) -> u16 {
        self.kind_meta.data_offset()
    }

    pub const fn group_count(&self) -> u16 {
        self.kind_meta.group_count()
    }

    /// Whether this primitive's immediate ancestor is a `Group`/`OptionGroup`/`Map`.
    /// See [`KindMeta::group`].
    pub const fn group(&self) -> bool {
        self.kind_meta.group()
    }

    /// Whether the ancestor context in [`Self::group`] forces topology
    /// (nested) treatment rather than the fixed region. See
    /// [`KindMeta::group_nest`].
    pub const fn group_nest(&self) -> bool {
        self.kind_meta.group_nest()
    }

    /// Get the nullable bitmap bit index for this field.
    /// Only nullable fixed fields (OptionPrimitive at nesting=0) have a non-zero index.
    pub const fn nullable_bit_index(&self) -> u16 {
        self.kind_meta.nullable_bit_index()
    }

    /// Check if this inode is any primitive kind (scalar or list, nullable or
    /// not) — i.e. it occupies bytes in the fixed region via `size()`,
    /// as opposed to Group/Map which don't.
    pub const fn is_primitive(&self) -> bool {
        matches!(self.kind_meta, KindMeta::Primitive { .. })
    }

    /// Check if this is a nullable fixed field (OptionPrimitive at nesting=0).
    ///
    /// Note: this alone doesn't rule out an `Option` scalar that's actually
    /// a topology item because some *ancestor* forces nested treatment
    /// (see [`Self::group_nest`]) — callers that need the real
    /// fixed-region-vs-topology answer should also check `!group_nest()`.
    pub const fn is_nullable_fixed(&self) -> bool {
        self.kind_meta.is_primitive_at_nesting_zero()
            && matches!(
                self.kind,
                WireKind::OptionBool
                    | WireKind::OptionI8
                    | WireKind::OptionI16
                    | WireKind::OptionI32
                    | WireKind::OptionI64
                    | WireKind::OptionU8
                    | WireKind::OptionU16
                    | WireKind::OptionU32
                    | WireKind::OptionU64
                    | WireKind::OptionF16
                    | WireKind::OptionF32
                    | WireKind::OptionF64
                    | WireKind::OptionTimestamp
            )
    }

    /// Check if this is a plain (non-list, non-ancestor-nested) `OptionGroup`
    /// — the group-level analogue of [`Self::is_nullable_fixed`]. Its own
    /// presence/absence gets a real bit in the shared nullable bitmap
    /// rather than being inferred from its members' nullability. A
    /// list-mode or ancestor-nested `OptionGroup` (`nesting() > 0` or
    /// `group_nest()`) tracks presence through its topology pointer
    /// instead, not this bitmap.
    pub const fn is_nullable_group(&self) -> bool {
        matches!(self.kind, WireKind::OptionGroup) && self.nesting() == 0 && !self.group_nest()
    }
}

#[cfg(test)]
mod tests {
    use super::super::kind::{KindMeta, WireKind};
    use super::Inode;

    /// Step 1 of the const-layout plan: `Inode`/`KindMeta` are now `Copy`
    /// and their accessors are `const fn`. This test only compiles if that
    /// is genuinely true — a `const { .. }` block can only call `const fn`s
    /// and only read from `Copy` values without moving them. If any of the
    /// accessors below regress to non-const (or `Inode`/`KindMeta` lose
    /// `Copy`), this test fails to compile, which *is* the test.
    #[test]
    fn test_accessors_usable_in_const() {
        const {
            let inode = Inode {
                kind: WireKind::U32,
                kind_meta: KindMeta::Primitive {
                    nesting: 1,
                    data_offset: 8,
                    topology_offset: 0,
                    fixed_size: 0,
                    nullable_bit_index: 0,
                    group: false,
                    group_nest: false,
                },
            };

            // WireKind
            assert!(inode.kind.byte_size() == 4);
            assert!(inode.kind.encode() == WireKind::U32 as u8);

            // KindMeta, via a standalone value (Copy lets us read it
            // multiple times without re-constructing).
            let meta = inode.kind_meta;
            assert!(meta.nesting() == 1);
            assert!(meta.topology_offset() == 0);
            assert!(meta.data_offset() == 8);
            assert!(meta.fixed_size() == 0);
            assert!(meta.group_count() == 0);
            assert!(meta.nullable_bit_index() == 0);
            assert!(!meta.is_primitive_at_nesting_zero());

            // Inode, reading `inode` itself multiple times relies on Copy.
            assert!(inode.nesting() == 1);
            assert!(inode.topology_offset() == 0);
            assert!(inode.data_offset() == 8);
            assert!(inode.fixed_size() == 0);
            assert!(inode.group_count() == 0);
            assert!(inode.nullable_bit_index() == 0);
            assert!(inode.alignment() == 4);
            assert!(inode.size::<u32>() == 8);
            assert!(inode.is_primitive());
            assert!(!inode.is_nullable_fixed());
        }
    }

    #[test]
    fn test_inode_encode_decode() {
        let inode = Inode {
            kind: WireKind::U32,
            kind_meta: KindMeta::Primitive {
                nesting: 1,
                fixed_size: 0,
                data_offset: 2048,
                topology_offset: 0,
                nullable_bit_index: 0,
                group: false,
                group_nest: false,
            },
        };

        let encoded = inode.encode();
        let decoded = Inode::decode(&encoded).expect("Inode decode failed");

        assert_eq!(decoded.kind, WireKind::U32);
        assert_eq!(
            decoded.kind_meta,
            KindMeta::Primitive {
                nesting: 1,
                fixed_size: 0,
                data_offset: 2048,
                topology_offset: 0,
                nullable_bit_index: 0,
                group: false,
                group_nest: false,
            }
        );
        assert_eq!(decoded.alignment(), 4); // Inner U32 element size
        assert_eq!(decoded.size::<u32>(), 8); // nesting=1 -> pointer pair, not 4*fixed_size
    }
}
