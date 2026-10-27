//! Four pure compatibility checks over `(inode table, name table)` — no
//! transport, no policy. See `docs/pyro-schema-crate-plan.md`, "Four
//! compatibility checks".
//!
//! | # | check | layout | names | answers |
//! | - | - | - | - | - |
//! | 1 | [`schema_hash`] | 8-byte hash | none | is this payload compatible with this pipe? |
//! | 2 | [`layout_eq`] | exact `==` | none | same, with no collision risk |
//! | 3 | [`schema_eq`] | exact `==` | all | do both ends mean the same fields, everywhere? |
//! | 4 | [`signature_eq`] | exact `==` | non-root only | same signature, allowing renamed parameters? |
//!
//! Docs never participate at any level. Policy — which level an end
//! requires, when it runs — is deliberately not this crate's business.

use crate::types::FieldNames;
use crate::wire::{Inode, InodeMeta, WireKind};

// ---------------------------------------------------------------------------
// Level 1 — Hash
// ---------------------------------------------------------------------------

const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

#[inline]
const fn fnv1a(mut hash: u64, bytes: &[u8]) -> u64 {
    let mut i = 0;
    while i < bytes.len() {
        hash ^= bytes[i] as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
        i += 1;
    }
    hash
}

#[inline]
const fn fnv1a_u32(hash: u64, v: u32) -> u64 {
    fnv1a(hash, &v.to_le_bytes())
}

#[inline]
const fn fnv1a_u16(hash: u64, v: u16) -> u64 {
    fnv1a(hash, &v.to_le_bytes())
}

#[inline]
const fn fnv1a_u8(mut hash: u64, v: u8) -> u64 {
    hash ^= v as u64;
    hash.wrapping_mul(FNV_PRIME)
}

/// Canonical 64-bit fingerprint of a schema's **type** — level 1 of the
/// four-level ladder. `const fn` so a derive can expose it as a compile-time
/// constant.
///
/// Hashed, in order: the whole encoded [`InodeMeta`] (so the structure word,
/// and any message-wide setting added to it later), the inode count, then
/// per inode every field that
/// contributes to the wire layout — the `kind` byte, `nesting`,
/// `data_offset`, `topology_offset`, `fixed_size`, `nullable_bit_index` and
/// `group_count`.
///
/// Field names are deliberately not hashed — a rename is not a wire-format
/// change, since names never appear in the row data (only in the header's
/// `fields_buf` metadata). This is a probabilistic check (FNV-1a 64): it
/// catches "these two ends were built against different schemas", a
/// configuration error, not an adversary. Levels 2-4 below are the
/// deterministic/named counterparts.
pub const fn schema_hash(meta: &InodeMeta, inodes: &[Inode]) -> u64 {
    let mut h = FNV_OFFSET_BASIS;

    h = fnv1a(h, &meta.encode());
    h = fnv1a_u32(h, inodes.len() as u32);
    let mut i = 0;
    while i < inodes.len() {
        let inode = &inodes[i];
        h = fnv1a_u8(h, inode.kind.encode());
        h = fnv1a_u8(h, inode.kind_meta.nesting());
        h = fnv1a_u16(h, inode.kind_meta.data_offset());
        h = fnv1a_u16(h, inode.kind_meta.topology_offset());
        h = fnv1a_u16(h, inode.kind_meta.fixed_size());
        h = fnv1a_u16(h, inode.kind_meta.nullable_bit_index());
        h = fnv1a_u16(h, inode.kind_meta.group_count());
        i += 1;
    }

    // A real schema must never hash to zero: zero is reserved for the unit
    // message, and letting a schema claim it would make the exemption
    // ambiguous. The nudge is deterministic, so it is still stable.
    if h == 0 { FNV_OFFSET_BASIS } else { h }
}

// ---------------------------------------------------------------------------
// Level 2 — Layout
// ---------------------------------------------------------------------------

/// Level 2: exact `==` over the raw inode tables, no names involved. The
/// lightweight signature check — an end that does not care what fields are
/// called compares this and stops.
pub fn layout_eq(a: &[Inode], b: &[Inode]) -> bool {
    a == b
}

// ---------------------------------------------------------------------------
// Root-span walk — shared by levels 3 and 4
// ---------------------------------------------------------------------------

/// Number of *additional* flat-name slots consumed by the subtree rooted at
/// `inodes[*idx]`, not counting a slot for the subtree's own name (callers
/// that need one add it themselves — see [`top_level_spans`]). Advances
/// `*idx` past every inode the subtree owns.
///
/// Mirrors `PyroType::name_count()`, computed from the inode table alone: a
/// `Group`/`OptionGroup`'s `count` children are each named (`1 +` their own
/// contribution), while a `Map`'s key/value are themselves unnamed (only
/// their own nested groups contribute).
fn subtree_name_slots(inodes: &[Inode], idx: &mut usize) -> usize {
    let inode = &inodes[*idx];
    *idx += 1;
    match inode.kind {
        WireKind::Group | WireKind::OptionGroup => {
            let count = inode.group_count() as usize;
            let mut total = 0;
            for _ in 0..count {
                total += 1 + subtree_name_slots(inodes, idx);
            }
            total
        }
        WireKind::Map => {
            let key = subtree_name_slots(inodes, idx);
            let value = subtree_name_slots(inodes, idx);
            key + value
        }
        _ => 0,
    }
}

/// Top-level field spans into the flat name array: `(start, len)` per
/// top-level field, where `start` is the index of that field's own
/// ("root") name slot and `len` is `1 + name_count()` of its type — its own
/// slot plus every nested `Group` name reachable from it.
///
/// A schema is encoded as its top-level fields' subtrees concatenated end to
/// end (no single wrapping root inode), so this walks the inode array the
/// same way [`subtree_name_slots`] walks a `Group`'s children.
pub fn top_level_spans(inodes: &[Inode]) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    let mut idx = 0;
    let mut name_pos = 0;
    while idx < inodes.len() {
        let len = 1 + subtree_name_slots(inodes, &mut idx);
        spans.push((name_pos, len));
        name_pos += len;
    }
    spans
}

/// The root-slot indices in the flat name array — the first slot of each
/// top-level field's span, i.e. one per top-level field and nothing else.
/// Root names are bindings (the caller's business); every other slot is
/// structure.
fn root_slots(inodes: &[Inode]) -> Vec<usize> {
    top_level_spans(inodes)
        .into_iter()
        .map(|(s, _)| s)
        .collect()
}

// ---------------------------------------------------------------------------
// Levels 3 & 4 — Exact / Signature
// ---------------------------------------------------------------------------

/// Level 3 ("Exact"): [`layout_eq`] plus every name slot equal, at every
/// depth. Docs are excluded — a reworded `///` comment can never make two
/// schemas incompatible.
pub fn schema_eq(
    a_inodes: &[Inode],
    a_names: &FieldNames<'_>,
    b_inodes: &[Inode],
    b_names: &FieldNames<'_>,
) -> bool {
    if !layout_eq(a_inodes, b_inodes) {
        return false;
    }
    if a_names.len() != b_names.len() {
        return false;
    }
    a_names.iter().eq(b_names.iter())
}

/// Level 4 ("Signature"): [`layout_eq`] plus every **non-root** name slot
/// equal — root (top-level field) names are free to differ, since a
/// top-level request row's field names are the caller's parameter-naming
/// choice, not part of the type's identity. Docs are excluded, as in
/// [`schema_eq`].
pub fn signature_eq(
    a_inodes: &[Inode],
    a_names: &FieldNames<'_>,
    b_inodes: &[Inode],
    b_names: &FieldNames<'_>,
) -> bool {
    if !layout_eq(a_inodes, b_inodes) {
        return false;
    }
    if a_names.len() != b_names.len() {
        return false;
    }
    let roots: std::collections::HashSet<usize> = root_slots(a_inodes).into_iter().collect();
    for i in 0..a_names.len() {
        if roots.contains(&i) {
            continue;
        }
        if a_names.get(i) != b_names.get(i) {
            return false;
        }
    }
    true
}

// ---------------------------------------------------------------------------
// Caller-side convenience
// ---------------------------------------------------------------------------

/// Which of the four checks an end has chosen to require. Carries no wire
/// encoding and no default — a convenience type for an RPC layer to store a
/// configured choice in, nothing more. Policy (which level, when it runs) is
/// entirely the RPC spec's business.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaCheck {
    Hash,
    Layout,
    Exact,
    Signature,
}

/// Dispatch to the check named by `level`. `a_hash`/`b_hash` are each side's
/// pre-computed [`schema_hash`] (the common case: already carried in a
/// message header, so this never recomputes it).
pub fn check(
    level: SchemaCheck,
    a_hash: u64,
    a_inodes: &[Inode],
    a_names: &FieldNames<'_>,
    b_hash: u64,
    b_inodes: &[Inode],
    b_names: &FieldNames<'_>,
) -> bool {
    match level {
        SchemaCheck::Hash => a_hash == b_hash,
        SchemaCheck::Layout => layout_eq(a_inodes, b_inodes),
        SchemaCheck::Exact => schema_eq(a_inodes, a_names, b_inodes, b_names),
        SchemaCheck::Signature => signature_eq(a_inodes, a_names, b_inodes, b_names),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bridge::SchemaParts;
    use crate::types::{PrimitiveDataType, PyroField, PyroSchema, PyroType};
    use crate::wire::KindMeta;

    fn schema_of(fields: Vec<PyroField<'static>>) -> (Vec<Inode>, PyroSchema<'static>) {
        let schema = PyroSchema::new(fields);
        let inodes = SchemaParts::from_schema(&schema).unwrap().build_inodes();
        (inodes, schema)
    }

    #[test]
    fn schema_hash_is_never_zero() {
        assert_ne!(schema_hash(&InodeMeta::DEFAULT, &[]), 0);
    }

    #[test]
    fn word_changes_the_hash() {
        use crate::word::Word;
        assert_ne!(
            schema_hash(&InodeMeta::new(Word::U16), &[]),
            schema_hash(&InodeMeta::new(Word::U32), &[]),
        );
    }

    #[test]
    fn schema_hash_is_const_evaluable() {
        const H: u64 = schema_hash(&InodeMeta::DEFAULT, &[]);
        assert_eq!(H, schema_hash(&InodeMeta::DEFAULT, &[]));
    }

    #[test]
    fn fixed_size_changes_the_hash() {
        let make = |fixed_size| {
            [Inode {
                kind: WireKind::U8,
                kind_meta: KindMeta::Primitive {
                    nesting: 0,
                    data_offset: 0,
                    topology_offset: 0,
                    fixed_size,
                    nullable_bit_index: 0,
                    group: false,
                    group_nest: false,
                },
            }]
        };
        assert_ne!(
            schema_hash(&InodeMeta::DEFAULT, &make(4)),
            schema_hash(&InodeMeta::DEFAULT, &make(16))
        );
    }

    #[test]
    fn nullable_bit_index_changes_the_hash() {
        let make = |nullable_bit_index| {
            [Inode {
                kind: WireKind::OptionU32,
                kind_meta: KindMeta::Primitive {
                    nesting: 0,
                    data_offset: 0,
                    topology_offset: 0,
                    fixed_size: 0,
                    nullable_bit_index,
                    group: false,
                    group_nest: false,
                },
            }]
        };
        assert_ne!(
            schema_hash(&InodeMeta::DEFAULT, &make(0)),
            schema_hash(&InodeMeta::DEFAULT, &make(3))
        );
    }

    #[test]
    fn group_count_changes_the_hash() {
        let make = |count| {
            [Inode {
                kind: WireKind::Group,
                kind_meta: KindMeta::Group {
                    nesting: 0,
                    count,
                    topology_offset: 0,
                    nullable_bit_index: 0,
                    group: false,
                    group_nest: false,
                },
            }]
        };
        assert_ne!(
            schema_hash(&InodeMeta::DEFAULT, &make(2)),
            schema_hash(&InodeMeta::DEFAULT, &make(3))
        );
    }

    #[test]
    fn levels_are_ordered() {
        // Exact ⟹ Signature ⟹ Layout ⟹ Hash, for identical schemas.
        let (inodes, schema) = schema_of(vec![
            PyroField::new("id", PyroType::PrimitiveScalar(PrimitiveDataType::I64)),
            PyroField::group("addr", vec![PyroField::new("street", PyroType::Str)]),
        ]);
        let names = schema.names().clone();
        assert!(schema_eq(&inodes, &names, &inodes, &names));
        assert!(signature_eq(&inodes, &names, &inodes, &names));
        assert!(layout_eq(&inodes, &inodes));
        assert_eq!(
            schema_hash(&InodeMeta::DEFAULT, &inodes),
            schema_hash(&InodeMeta::DEFAULT, &inodes)
        );
    }

    #[test]
    fn renaming_root_name_passes_signature_fails_exact() {
        let (inodes_a, schema_a) = schema_of(vec![PyroField::new(
            "count",
            PyroType::PrimitiveScalar(PrimitiveDataType::U32),
        )]);
        let (inodes_b, schema_b) = schema_of(vec![PyroField::new(
            "n",
            PyroType::PrimitiveScalar(PrimitiveDataType::U32),
        )]);
        assert!(signature_eq(
            &inodes_a,
            schema_a.names(),
            &inodes_b,
            schema_b.names()
        ));
        assert!(!schema_eq(
            &inodes_a,
            schema_a.names(),
            &inodes_b,
            schema_b.names()
        ));
    }

    #[test]
    fn renaming_nested_name_fails_both_exact_and_signature() {
        let (inodes_a, schema_a) = schema_of(vec![PyroField::group(
            "addr",
            vec![PyroField::new("street", PyroType::Str)],
        )]);
        let (inodes_b, schema_b) = schema_of(vec![PyroField::group(
            "addr",
            vec![PyroField::new("road", PyroType::Str)],
        )]);
        assert!(!signature_eq(
            &inodes_a,
            schema_a.names(),
            &inodes_b,
            schema_b.names()
        ));
        assert!(!schema_eq(
            &inodes_a,
            schema_a.names(),
            &inodes_b,
            schema_b.names()
        ));
    }

    #[test]
    fn changing_a_field_type_fails_all_four() {
        let (inodes_a, schema_a) = schema_of(vec![PyroField::new(
            "id",
            PyroType::PrimitiveScalar(PrimitiveDataType::I64),
        )]);
        let (inodes_b, schema_b) = schema_of(vec![PyroField::new(
            "id",
            PyroType::PrimitiveScalar(PrimitiveDataType::I32),
        )]);
        assert_ne!(
            schema_hash(&InodeMeta::DEFAULT, &inodes_a),
            schema_hash(&InodeMeta::DEFAULT, &inodes_b)
        );
        assert!(!layout_eq(&inodes_a, &inodes_b));
        assert!(!schema_eq(
            &inodes_a,
            schema_a.names(),
            &inodes_b,
            schema_b.names()
        ));
        assert!(!signature_eq(
            &inodes_a,
            schema_a.names(),
            &inodes_b,
            schema_b.names()
        ));
    }

    /// Swapping two top-level fields of the *same* type passes every level
    /// (nothing distinguishes them once names are out of scope at level 2,
    /// and level 4 doesn't look at root names at all) — swapping two of
    /// *different* types fails Layout. This is correct, intended behavior.
    #[test]
    fn signature_is_positional() {
        let (same_type_a, _) = schema_of(vec![
            PyroField::new("a", PyroType::PrimitiveScalar(PrimitiveDataType::I64)),
            PyroField::new("b", PyroType::PrimitiveScalar(PrimitiveDataType::I64)),
        ]);
        let (same_type_b, _) = schema_of(vec![
            PyroField::new("b", PyroType::PrimitiveScalar(PrimitiveDataType::I64)),
            PyroField::new("a", PyroType::PrimitiveScalar(PrimitiveDataType::I64)),
        ]);
        assert!(layout_eq(&same_type_a, &same_type_b));

        let (diff_type_a, _) = schema_of(vec![
            PyroField::new("a", PyroType::PrimitiveScalar(PrimitiveDataType::I64)),
            PyroField::new("b", PyroType::Str),
        ]);
        let (diff_type_b, _) = schema_of(vec![
            PyroField::new("b", PyroType::Str),
            PyroField::new("a", PyroType::PrimitiveScalar(PrimitiveDataType::I64)),
        ]);
        assert!(!layout_eq(&diff_type_a, &diff_type_b));
    }

    #[test]
    fn hash_agrees_with_layout() {
        let (inodes, _) = schema_of(vec![PyroField::new(
            "id",
            PyroType::PrimitiveScalar(PrimitiveDataType::I64),
        )]);
        let other = inodes.clone();
        assert!(layout_eq(&inodes, &other));
        assert_eq!(
            schema_hash(&InodeMeta::DEFAULT, &inodes),
            schema_hash(&InodeMeta::DEFAULT, &other)
        );
    }
}
