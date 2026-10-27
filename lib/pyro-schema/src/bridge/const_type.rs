//! Const-eval `Inode` → [`PyroType`] decoding.
//!
//! A `const fn` can't allocate, so it can't return a `PyroType` whose
//! `Bow`/`Cow` children are owned — and it has nowhere `'static` to borrow
//! them from either. The children live in an *arena*: a `static` array of
//! `PyroType` nodes that [`inodes_to_type`] fills while borrowing from that
//! same static (a static may take references to itself in its own
//! initializer; it just can't read itself).
//!
//! ```ignore
//! static ARENA: [PyroType; type_node_count(INODES)] = inodes_to_type(INODES, &ARENA);
//! const TYPE: PyroType = arena_root(&ARENA);
//! ```
//!
//! [`const_pyro_type!`](crate::const_pyro_type) wraps exactly that.
//!
//! Arena layout: slot 0 is the root `Group` over the top-level fields, whose
//! children are slots `1..=k`. Every other node is bump-allocated so that
//! each `Group`'s children are contiguous (a `Cow::Borrowed` slice), each
//! `Map`'s key/value are adjacent, and each `List` points at the next node
//! of its wrapper chain.
//!
//! Decodes exactly like `SchemaParts::parse_type`; the tests hold the two
//! to each other.

use std::borrow::Cow;
use std::mem;

use crate::bow::Bow;
use crate::types::{PrimitiveDataType, PyroType};
use crate::wire::{Inode, KindMeta, WireKind};

use super::wire_kind_as_primitive_data_type;

/// Number of arena slots [`inodes_to_type`] fills for `inodes`: one per
/// `PyroType` node reachable from the top-level fields, plus the root
/// `Group`.
pub const fn type_node_count(inodes: &[Inode]) -> usize {
    let mut nodes = 1;
    let mut i = 0;
    while i < inodes.len() {
        let (n, next) = subtree(inodes, i);
        nodes += n;
        i = next;
    }
    nodes
}

/// Decode `inodes` into `arena` — which must be the very static this call
/// initializes — returning its contents. Slot 0 holds the root `Group`;
/// read it back out with [`arena_root`].
///
/// Panics (a compile error in const context) if `N` isn't
/// [`type_node_count`]`(inodes)` or `inodes` isn't a well-formed forest.
pub const fn inodes_to_type<const N: usize>(
    inodes: &[Inode],
    arena: &'static [PyroType; N],
) -> [PyroType; N] {
    let mut top_level = 0;
    let mut i = 0;
    while i < inodes.len() {
        i = subtree(inodes, i).1;
        top_level += 1;
    }

    let mut out = [const { PyroType::Unknown }; N];
    let mut fill = Fill {
        inodes,
        arena,
        out: &mut out,
        next: 1 + top_level,
    };
    fill.put(0, PyroType::Group(Cow::Borrowed(fill.slice(1, top_level))));

    let mut i = 0;
    let mut root = 0;
    while root < top_level {
        i = fill.node(i, 1 + root);
        root += 1;
    }
    assert!(
        fill.next == N,
        "arena length must be type_node_count(inodes)"
    );
    out
}

/// The root `Group` [`inodes_to_type`] left in slot 0 of `arena`, by value.
pub const fn arena_root(arena: &'static [PyroType]) -> PyroType {
    match &arena[0] {
        PyroType::Group(Cow::Borrowed(fields)) => PyroType::Group(Cow::Borrowed(fields)),
        _ => panic!("arena slot 0 is not a borrowed root Group"),
    }
}

/// A `PyroType` decoded from a `const` inode slice, as a `const` value.
///
/// ```ignore
/// const TYPE: PyroType = const_pyro_type!(<MyStruct as Layout>::RAW);
/// ```
///
/// The argument lands in a nested `static`, so it can't mention `Self` or
/// outer generics — name the concrete type.
#[macro_export]
macro_rules! const_pyro_type {
    ($inodes:expr) => {{
        const __PYRO_INODES: &[$crate::Inode] = $inodes;
        static __PYRO_ARENA: [$crate::PyroType; $crate::type_node_count(__PYRO_INODES)] =
            $crate::inodes_to_type(__PYRO_INODES, &__PYRO_ARENA);
        $crate::arena_root(&__PYRO_ARENA)
    }};
}

/// `(nodes, next)`: the number of `PyroType` nodes the subtree rooted at
/// `inodes[i]` decodes to, and the index just past that subtree.
const fn subtree(inodes: &[Inode], i: usize) -> (usize, usize) {
    let inode = inodes[i];
    let mut nodes = list_wrappers(&inode) as usize + 1;
    let mut next = i + 1;
    let children = match inode.kind_meta {
        KindMeta::Group { count, .. } => count as usize,
        KindMeta::Map { .. } => 2,
        KindMeta::Primitive { .. } => 0,
    };
    let mut c = 0;
    while c < children {
        let (n, after) = subtree(inodes, next);
        nodes += n;
        next = after;
        c += 1;
    }
    (nodes, next)
}

/// How many `PyroType::List` layers wrap this inode's base type.
const fn list_wrappers(inode: &Inode) -> u8 {
    match inode.kind_meta {
        KindMeta::Primitive {
            nesting,
            fixed_size,
            ..
        } => match inode.kind {
            // Only a plain `PrimitiveList` already carries the first layer
            // (`SchemaParts::push_node` writes it as `nesting: 1`). A fixed
            // list, an option scalar, a string and a blob all sit at
            // `nesting: 0` on their own.
            kind if fixed_size == 0 && wire_kind_as_primitive_data_type(kind).is_some() => {
                nesting.saturating_sub(1)
            }
            _ => nesting,
        },
        KindMeta::Group { nesting, .. } | KindMeta::Map { nesting, .. } => nesting,
    }
}

/// The base type of a non-`Group`, non-`Map` inode.
const fn leaf(inode: &Inode) -> PyroType {
    let (nesting, fixed_size) = (inode.nesting(), inode.fixed_size() as usize);
    match inode.kind {
        WireKind::Unknown => PyroType::Unknown,
        WireKind::Str => PyroType::Str,
        WireKind::Bytes => PyroType::Bytes,
        kind => {
            if let Some(pdt) = wire_kind_as_primitive_data_type(kind) {
                if fixed_size > 0 {
                    PyroType::PrimitiveFixedList(pdt, fixed_size)
                } else if nesting > 0 {
                    PyroType::PrimitiveList(pdt)
                } else {
                    PyroType::PrimitiveScalar(pdt)
                }
            } else if let Some(pdt) = option_wire_kind_as_primitive_data_type(kind) {
                if fixed_size > 0 {
                    PyroType::OptionPrimitiveFixedList(pdt, fixed_size)
                } else {
                    PyroType::OptionPrimitiveScalar(pdt)
                }
            } else {
                panic!("Group/OptionGroup/Map inode is not a leaf")
            }
        }
    }
}

const fn option_wire_kind_as_primitive_data_type(kind: WireKind) -> Option<PrimitiveDataType> {
    match kind {
        WireKind::OptionBool => Some(PrimitiveDataType::Bool),
        WireKind::OptionI8 => Some(PrimitiveDataType::I8),
        WireKind::OptionI16 => Some(PrimitiveDataType::I16),
        WireKind::OptionI32 => Some(PrimitiveDataType::I32),
        WireKind::OptionI64 => Some(PrimitiveDataType::I64),
        WireKind::OptionU8 => Some(PrimitiveDataType::U8),
        WireKind::OptionU16 => Some(PrimitiveDataType::U16),
        WireKind::OptionU32 => Some(PrimitiveDataType::U32),
        WireKind::OptionU64 => Some(PrimitiveDataType::U64),
        WireKind::OptionF16 => Some(PrimitiveDataType::F16),
        WireKind::OptionF32 => Some(PrimitiveDataType::F32),
        WireKind::OptionF64 => Some(PrimitiveDataType::F64),
        WireKind::OptionTimestamp => Some(PrimitiveDataType::Timestamp),
        _ => None,
    }
}

/// In-progress arena fill: `out` is what's being written, `arena` is the
/// finished static the written nodes borrow from, `next` the bump pointer.
struct Fill<'i, 'o, const N: usize> {
    inodes: &'i [Inode],
    arena: &'static [PyroType; N],
    out: &'o mut [PyroType; N],
    next: usize,
}

impl<const N: usize> Fill<'_, '_, N> {
    /// Write `ty` into `slot`. Assignment would drop the placeholder, which
    /// const eval can't do — so swap it out and forget it (it's `Unknown`).
    const fn put(&mut self, slot: usize, ty: PyroType) {
        mem::forget(mem::replace(&mut self.out[slot], ty));
    }

    const fn alloc(&mut self, len: usize) -> usize {
        let start = self.next;
        self.next += len;
        start
    }

    const fn slice(&self, start: usize, len: usize) -> &'static [PyroType] {
        let arena: &'static [PyroType] = self.arena;
        arena.split_at(start).1.split_at(len).0
    }

    /// Decode the subtree at `inodes[i]` into `slot` (plus freshly allocated
    /// slots for its inner nodes); returns the index past the subtree.
    const fn node(&mut self, i: usize, slot: usize) -> usize {
        let inode = self.inodes[i];

        let mut slot = slot;
        let mut layers = list_wrappers(&inode);
        while layers > 0 {
            let inner = self.alloc(1);
            self.put(slot, PyroType::List(Bow::Borrowed(&self.arena[inner])));
            slot = inner;
            layers -= 1;
        }

        match inode.kind_meta {
            KindMeta::Group { count, .. } => {
                let count = count as usize;
                let start = self.alloc(count);
                let fields = Cow::Borrowed(self.slice(start, count));
                let ty = match inode.kind {
                    WireKind::OptionGroup => PyroType::OptionGroup(fields),
                    _ => PyroType::Group(fields),
                };
                self.put(slot, ty);

                let mut next = i + 1;
                let mut c = 0;
                while c < count {
                    next = self.node(next, start + c);
                    c += 1;
                }
                next
            }
            KindMeta::Map { .. } => {
                let kv = self.alloc(2);
                self.put(
                    slot,
                    PyroType::Map {
                        key: Bow::Borrowed(&self.arena[kv]),
                        value: Bow::Borrowed(&self.arena[kv + 1]),
                    },
                );
                let next = self.node(i + 1, kv);
                self.node(next, kv + 1)
            }
            KindMeta::Primitive { .. } => {
                self.put(slot, leaf(&inode));
                i + 1
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bridge::SchemaParts;

    const fn prim(kind: WireKind, nesting: u8, fixed_size: u16) -> Inode {
        Inode {
            kind,
            kind_meta: KindMeta::Primitive {
                nesting,
                data_offset: 0,
                topology_offset: 0,
                fixed_size,
                nullable_bit_index: 0,
                group: false,
                group_nest: false,
            },
        }
    }

    const fn group(kind: WireKind, nesting: u8, count: u16) -> Inode {
        Inode {
            kind,
            kind_meta: KindMeta::Group {
                nesting,
                count,
                topology_offset: 0,
                nullable_bit_index: 0,
                group: false,
                group_nest: false,
            },
        }
    }

    const fn map(nesting: u8) -> Inode {
        Inode {
            kind: WireKind::Map,
            kind_meta: KindMeta::Map {
                nesting,
                topology_offset: 0,
                group: false,
                group_nest: false,
            },
        }
    }

    /// One of everything: every leaf shape, list wrappers on each kind of
    /// base, nested (optional) groups, and a map with a group value.
    const INODES: &[Inode] = &[
        prim(WireKind::U32, 0, 0),
        prim(WireKind::F32, 0, 4),
        prim(WireKind::I16, 2, 0),
        prim(WireKind::U8, 2, 3),
        prim(WireKind::OptionI64, 0, 0),
        prim(WireKind::OptionF64, 0, 2),
        prim(WireKind::OptionBool, 2, 0),
        prim(WireKind::Str, 0, 0),
        prim(WireKind::Bytes, 1, 0),
        prim(WireKind::Timestamp, 0, 0),
        prim(WireKind::OptionTimestamp, 2, 0),
        prim(WireKind::Unknown, 0, 0),
        group(WireKind::Group, 1, 2),
        prim(WireKind::Str, 0, 0),
        group(WireKind::OptionGroup, 0, 2),
        prim(WireKind::U16, 0, 0),
        prim(WireKind::Str, 1, 0),
        map(1),
        prim(WireKind::Str, 0, 0),
        group(WireKind::Group, 0, 1),
        prim(WireKind::U64, 1, 0),
    ];

    const TYPE: PyroType = crate::const_pyro_type!(INODES);

    #[test]
    fn const_decode_matches_runtime_decode() {
        let runtime = PyroType::Group(SchemaParts::parse_top_level(INODES).unwrap().into());
        assert_eq!(TYPE, runtime);
    }

    #[test]
    fn empty_inodes_decode_to_empty_group() {
        const EMPTY: PyroType = crate::const_pyro_type!(&[]);
        assert_eq!(EMPTY, PyroType::Group(Cow::Borrowed(&[])));
    }

    #[test]
    fn arena_is_exactly_sized() {
        static ARENA: [PyroType; type_node_count(INODES)] = inodes_to_type(INODES, &ARENA);
        let unknowns = ARENA
            .iter()
            .filter(|t| matches!(t, PyroType::Unknown))
            .count();
        assert_eq!(
            unknowns, 1,
            "only the WireKind::Unknown leaf decodes to Unknown"
        );
    }
}
