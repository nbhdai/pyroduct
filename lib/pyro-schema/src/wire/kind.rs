macro_rules! define_wire_kind {
    ($( $name:ident = $val:expr ),* $(,)?) => {
        /// A 7-bit type discriminant for the wire format.
        ///
        /// Values must be in 1..=127.  Bit 7 is reserved for the nullable
        /// flag and is never part of the enum value itself.
        #[repr(u8)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum WireKind {
            $( $name = $val, )*
        }

        impl WireKind {
            /// Parse the low 7 bits of a raw byte into a `WireKind`.
            /// The nullable flag (bit 7) is stripped before matching.
            pub fn parse(raw: u8) -> Result<WireKind, u8> {
                let kind = match raw & 0x7F {
                    $( $val => WireKind::$name, )*
                    _ => return Err(raw),
                };
                Ok(kind)
            }

            /// Encode this kind into a single byte.
            pub const fn encode(self) -> u8 {
                self as u8
            }
        }
    };
}

define_wire_kind! {
    // Scalars
    Unknown      = 1,
    Bool      = 2,
    I8        = 3,
    I16       = 4,
    I32       = 5,
    I64       = 6,
    //I128    = 7,
    U8        = 8,
    U16       = 9,
    U32       = 10,
    U64       = 11,
    //U128    = 12,
    F16       = 13,
    F32       = 14,
    F64       = 15,
    Timestamp = 16,

    OptionBool      = 32,
    OptionI8        = 33,
    OptionI16       = 34,
    OptionI32       = 35,
    OptionI64       = 36,
    //OptionI128    = 37,
    OptionU8        = 38,
    OptionU16       = 39,
    OptionU32       = 40,
    OptionU64       = 41,
    //OptionU128    = 42,
    OptionF16       = 43,
    OptionF32       = 44,
    OptionF64       = 45,
    OptionTimestamp = 46,

    Str       = 50,
    Bytes       = 51,

    // Structural / nested
    Group     = 64,
    OptionGroup     = 65,
    Map       = 66,
    // List is not included
}

impl WireKind {
    pub const fn byte_size(&self) -> usize {
        match self {
            WireKind::Unknown => 0,
            WireKind::Bool | WireKind::OptionBool => 1,
            WireKind::I8 | WireKind::OptionI8 => 1,
            WireKind::I16 | WireKind::OptionI16 => 2,
            WireKind::I32 | WireKind::OptionI32 => 4,
            WireKind::I64 | WireKind::OptionI64 => 8,
            WireKind::U8 | WireKind::OptionU8 => 1,
            WireKind::U16 | WireKind::OptionU16 => 2,
            WireKind::U32 | WireKind::OptionU32 => 4,
            WireKind::U64 | WireKind::OptionU64 => 8,
            WireKind::F16 | WireKind::OptionF16 => 2,
            WireKind::F32 | WireKind::OptionF32 => 4,
            WireKind::F64 | WireKind::OptionF64 => 8,
            WireKind::Timestamp | WireKind::OptionTimestamp => 16,
            WireKind::Str | WireKind::Bytes => 1,
            WireKind::Group | WireKind::OptionGroup | WireKind::Map => 0,
        }
    }
}

/// This is encoded in 88 bits, or 11 bytes
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KindMeta {
    Primitive {
        // How deeply does this nest? 1 means List<T>, 2 means List<List<T>>
        nesting: u8,
        // The offset to the fixed pointer.
        data_offset: u16,
        // The offset to the topology pointer. Needed if nesting > 0
        topology_offset: u16,
        // Temporary
        fixed_size: u16,
        // Bit index within the nullable bitmap (only for nullable fixed fields).
        // The bitmap is stored at the end of the fixed section.
        nullable_bit_index: u16,
        // Are we in a group or map?
        group: bool,
        // Is that group or map nested? (maps are always nested)
        group_nest: bool,
    },
    Group {
        // How deeply does this nest? 1 means List<Group<...>>, 2 means List<List<Group<...>>>
        nesting: u8,
        // How many elements are in the group
        count: u16,
        // The offset to the topology pointer. Needed if nesting > 0
        topology_offset: u16,
        // Bit index within the nullable bitmap (only for nullable fixed fields).
        // The bitmap is stored at the end of the fixed section.
        nullable_bit_index: u16,
        // Are we in a group or map?
        group: bool,
        // Is that group or map nested? (maps are always nested)
        group_nest: bool,
    },
    Map {
        // How deeply does this nest? 1 means List<Map<...>>, 2 means List<List<Map<...>>>
        nesting: u8,
        // The offset to the topology pointer. Needed if nesting > 0
        topology_offset: u16,
        // We don't have a fixed pointer for maps, they're virtual
        // Are we in a group or map?
        group: bool,
        // Is that group or map nested? (maps are always nested)
        group_nest: bool,
    },
}

impl KindMeta {
    pub const fn nesting(&self) -> u8 {
        match self {
            KindMeta::Primitive { nesting, .. } => *nesting,
            KindMeta::Group { nesting, .. } => *nesting,
            KindMeta::Map { nesting, .. } => *nesting,
        }
    }

    pub const fn topology_offset(&self) -> u16 {
        match self {
            KindMeta::Primitive {
                topology_offset, ..
            } => *topology_offset,
            KindMeta::Group {
                topology_offset, ..
            } => *topology_offset,
            KindMeta::Map {
                topology_offset, ..
            } => *topology_offset,
        }
    }

    pub const fn data_offset(&self) -> u16 {
        match self {
            KindMeta::Primitive { data_offset, .. } => *data_offset,
            _ => 0,
        }
    }

    pub const fn fixed_size(&self) -> u16 {
        match self {
            KindMeta::Primitive { fixed_size, .. } => *fixed_size,
            _ => 0,
        }
    }

    pub const fn group_count(&self) -> u16 {
        match self {
            KindMeta::Group { count, .. } => *count,
            _ => 0,
        }
    }

    /// Get the nullable bitmap bit index for this field.
    ///
    /// Non-zero for a nullable fixed field (`OptionPrimitive` at
    /// nesting=0, not ancestor-nested) *or* a plain (non-list,
    /// non-ancestor-nested) `OptionGroup` — a group's own presence/absence
    /// gets a real bit here too, same shared bitmap, rather than being
    /// inferred from its members' nullability.
    pub const fn nullable_bit_index(&self) -> u16 {
        match self {
            KindMeta::Primitive {
                nullable_bit_index, ..
            } => *nullable_bit_index,
            KindMeta::Group {
                nullable_bit_index, ..
            } => *nullable_bit_index,
            KindMeta::Map { .. } => 0,
        }
    }

    /// Check if this is a primitive at nesting=0.
    /// The actual nullable check is done at the Inode level by checking the WireKind.
    pub const fn is_primitive_at_nesting_zero(&self) -> bool {
        matches!(self, KindMeta::Primitive { nesting: 0, .. })
    }

    /// Whether this node's immediate ancestor is a `Group`/`OptionGroup`/`Map`
    /// at all (regardless of whether that container is itself list-nested).
    /// `false` for a root-level field.
    pub const fn group(&self) -> bool {
        match self {
            KindMeta::Primitive { group, .. } => *group,
            KindMeta::Group { group, .. } => *group,
            KindMeta::Map { group, .. } => *group,
        }
    }

    /// Whether the enclosing group/map context (see [`Self::group`]) means
    /// this node needs topology (nested) treatment rather than being
    /// inlined into the fixed region — i.e. some ancestor contributes
    /// nesting, either because an ancestor `Group`/`OptionGroup` is itself
    /// list-nested, or unconditionally because the ancestor is a `Map`
    /// (a map's key/value are always stored as nested lists of its pairs,
    /// regardless of whether the map itself is wrapped in a `List<>`).
    /// Only meaningful when [`Self::group`] is `true`; `false` for a
    /// root-level field.
    pub const fn group_nest(&self) -> bool {
        match self {
            KindMeta::Primitive { group_nest, .. } => *group_nest,
            KindMeta::Group { group_nest, .. } => *group_nest,
            KindMeta::Map { group_nest, .. } => *group_nest,
        }
    }

    pub fn encode(&self) -> [u8; 11] {
        let mut buf = [0u8; 11];
        match self {
            KindMeta::Primitive {
                nesting,
                data_offset,
                topology_offset,
                fixed_size,
                nullable_bit_index,
                group,
                group_nest,
            } => {
                buf[0] = *nesting;
                buf[1..3].copy_from_slice(&data_offset.to_le_bytes());
                buf[3..5].copy_from_slice(&topology_offset.to_le_bytes());
                buf[5..7].copy_from_slice(&fixed_size.to_le_bytes());
                buf[7..9].copy_from_slice(&nullable_bit_index.to_le_bytes());
                buf[9] = (*group as u8) | ((*group_nest as u8) << 1);
            }
            KindMeta::Group {
                count,
                nesting,
                topology_offset,
                nullable_bit_index,
                group,
                group_nest,
            } => {
                buf[0] = *nesting;
                buf[1..3].copy_from_slice(&count.to_le_bytes());
                buf[3..5].copy_from_slice(&topology_offset.to_le_bytes());
                buf[7..9].copy_from_slice(&nullable_bit_index.to_le_bytes());
                buf[9] = (*group as u8) | ((*group_nest as u8) << 1);
            }
            KindMeta::Map {
                nesting,
                topology_offset,
                group,
                group_nest,
            } => {
                buf[0] = *nesting;
                buf[3..5].copy_from_slice(&topology_offset.to_le_bytes());
                buf[9] = (*group as u8) | ((*group_nest as u8) << 1);
            }
        }
        buf
    }

    pub fn decode(kind: WireKind, bytes: [u8; 11]) -> Self {
        match kind {
            WireKind::Group | WireKind::OptionGroup => KindMeta::Group {
                nesting: bytes[0],
                count: u16::from_le_bytes([bytes[1], bytes[2]]),
                topology_offset: u16::from_le_bytes([bytes[3], bytes[4]]),
                nullable_bit_index: u16::from_le_bytes([bytes[7], bytes[8]]),
                group: bytes[9] & 0b01 != 0,
                group_nest: bytes[9] & 0b10 != 0,
            },
            WireKind::Map => KindMeta::Map {
                nesting: bytes[0],
                topology_offset: u16::from_le_bytes([bytes[3], bytes[4]]),
                group: bytes[9] & 0b01 != 0,
                group_nest: bytes[9] & 0b10 != 0,
            },
            _ => KindMeta::Primitive {
                nesting: bytes[0],
                data_offset: u16::from_le_bytes([bytes[1], bytes[2]]),
                topology_offset: u16::from_le_bytes([bytes[3], bytes[4]]),
                fixed_size: u16::from_le_bytes([bytes[5], bytes[6]]),
                nullable_bit_index: u16::from_le_bytes([bytes[7], bytes[8]]),
                group: bytes[9] & 0b01 != 0,
                group_nest: bytes[9] & 0b10 != 0,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Round-trip `group`/`group_nest` through encode/decode for every
    /// combination, alongside the rest of a `Primitive`'s fields, to catch
    /// a stray bit getting dropped or misplaced in the packed byte.
    #[test]
    fn primitive_group_flags_round_trip() {
        for group in [false, true] {
            for group_nest in [false, true] {
                let meta = KindMeta::Primitive {
                    nesting: 2,
                    data_offset: 0x1234,
                    topology_offset: 0x5678,
                    fixed_size: 0x9abc,
                    nullable_bit_index: 7,
                    group,
                    group_nest,
                };
                let decoded = KindMeta::decode(WireKind::U32, meta.encode());
                assert_eq!(decoded, meta);
                assert_eq!(decoded.group(), group);
                assert_eq!(decoded.group_nest(), group_nest);
            }
        }
    }

    /// Same round-trip, for `Group`'s `nullable_bit_index`/`group`/`group_nest`
    /// — and for both `WireKind::Group` and `WireKind::OptionGroup`, since
    /// both discriminants decode into the same `KindMeta::Group` shape.
    #[test]
    fn group_nullable_and_group_flags_round_trip() {
        for kind in [WireKind::Group, WireKind::OptionGroup] {
            for group in [false, true] {
                for group_nest in [false, true] {
                    let meta = KindMeta::Group {
                        nesting: 1,
                        count: 3,
                        topology_offset: 0x5678,
                        nullable_bit_index: 5,
                        group,
                        group_nest,
                    };
                    let decoded = KindMeta::decode(kind, meta.encode());
                    assert_eq!(decoded, meta, "kind={kind:?}");
                    assert_eq!(decoded.nullable_bit_index(), 5, "kind={kind:?}");
                    assert_eq!(decoded.group(), group, "kind={kind:?}");
                    assert_eq!(decoded.group_nest(), group_nest, "kind={kind:?}");
                }
            }
        }
    }

    /// Same round-trip, for `Map`'s `group`/`group_nest` (it has no
    /// `nullable_bit_index` — there's no `OptionMap` discriminant).
    #[test]
    fn map_group_flags_round_trip() {
        for group in [false, true] {
            for group_nest in [false, true] {
                let meta = KindMeta::Map {
                    nesting: 1,
                    topology_offset: 0x5678,
                    group,
                    group_nest,
                };
                let decoded = KindMeta::decode(WireKind::Map, meta.encode());
                assert_eq!(decoded, meta);
                assert_eq!(decoded.group(), group);
                assert_eq!(decoded.group_nest(), group_nest);
            }
        }
    }
}
