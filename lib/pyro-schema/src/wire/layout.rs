use crate::StructureWord;

use super::inode::Inode;
use super::kind::{KindMeta, WireKind};
use super::validity::nullable_validity_start;

/// Hard cap on the number of inodes a single schema may contain.
///
/// [`layout_in_place`] needs a bound on `inodes.len()` so it can be a
/// genuine `const fn` (no heap allocation, no unbounded const-eval step
/// budget). Chosen generously — real schemas are nowhere close to this.
pub const MAX_INODES: usize = 1024;

/// Hard cap on how many `Group`/`OptionGroup`/`Map` levels may be nested
/// inside one another.
///
/// [`layout_in_place`] tracks nesting depth with a fixed-size scratch array
/// instead of a `Vec` (so it can run at const-eval time), and this is that
/// array's length. Chosen generously — real schemas nest a handful of
/// levels deep at most.
pub const MAX_GROUP_DEPTH: usize = 64;

/// Sentinel written into a fixed (non-pointer) primitive's `data_offset`
/// during the topology pass below, marking "this belongs in the fixed
/// region but hasn't been assigned a real offset yet." The ten bucket
/// passes look for this exact value to find their candidates without any
/// separate scratch collection.
///
/// Safe from collision with a genuine assigned offset: every real offset
/// assigned by this function is bounded by `MAX_INODES` inodes at a few
/// dozen bytes apiece, nowhere near `u16::MAX`.
const FIXED_ITEM_PENDING: u16 = u16::MAX;

/// The alignment the fixed-region-plus-bitmap total must be padded to, so
/// the dynamic region that follows starts on a boundary matching whatever
/// gets written there first.
///
/// The writer emits the dynamic region as every item's topology arrays
/// first, then raw data in descending alignment order (16 → 8 → 4 → 2 → 1)
/// — see `ValueWriter::write`. So:
///
/// - If *any* entry puts a topology array of offset pairs in the dynamic
///   region — a `Group`/`OptionGroup`/`Map`, a nested `Str`/`Bytes`
///   (per-element offsets ahead of the raw bytes), or a primitive whose
///   own list-of-lists shape needs the double-pointer form
///   (`nesting() >= 2`, or `nesting() == 1` with `group_nest()` — see
///   `layout_topology`'s own double-pointer condition) — that array is
///   written first, so the answer is 4. E.g. `Vec<Timestamp>` +
///   `Vec<Vec<u8>>` pads to 4.
/// - Otherwise the first thing written is the highest-aligned raw payload,
///   so the answer is the max `inode.alignment()` over them (a bare
///   `Str`/`Bytes` contributes 1). E.g. `Vec<u8>` + `Vec<Timestamp>` pads
///   to 16, regardless of field order.
///
/// Returns 1 (no padding) if there's no dynamic region at all.
const fn dynamic_region_alignment<W: StructureWord>(inodes: &[Inode]) -> u16 {
    let mut max_data_align: u16 = 1;
    let mut i = 0;
    while i < inodes.len() {
        let inode = inodes[i];
        match inode.kind {
            WireKind::Unknown => {}
            WireKind::Bool
            | WireKind::I8
            | WireKind::I16
            | WireKind::I32
            | WireKind::I64
            | WireKind::U8
            | WireKind::U16
            | WireKind::U32
            | WireKind::U64
            | WireKind::F16
            | WireKind::F32
            | WireKind::F64
            | WireKind::Timestamp
            | WireKind::OptionBool
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
            | WireKind::OptionTimestamp => {
                if inode.nesting() >= 2 || inode.group_nest() {
                    return W::SIZE as u16;
                }
                if inode.nesting() == 1 && inode.alignment() as u16 > max_data_align {
                    max_data_align = inode.alignment() as u16;
                }
            }
            WireKind::Str | WireKind::Bytes => {
                if inode.nesting() > 0 || inode.group_nest() {
                    return W::SIZE as u16;
                }
            }
            WireKind::Group => {
                if inode.nesting() > 0 {
                    return W::SIZE as u16;
                }
            }
            WireKind::OptionGroup => {
                if inode.nesting() > 0 || inode.group_nest() {
                    return W::SIZE as u16;
                }
            }
            WireKind::Map => return W::SIZE as u16,
        }
        i += 1;
    }
    max_data_align
}

/// The most padding the writers can put in the dynamic region of a message
/// laid out as `inodes`, whatever its values.
///
/// The dynamic region is written lane by lane — every inode's structure (its
/// topology tiers, an option group's mask), then its raw data in descending
/// alignment order (16 → 8 → 4 → 2 → 1) — and within a lane in inode order.
/// Each write pads to its lane's alignment first, and pads only as far as
/// the one before it may have left the lane short: words and plain runs end
/// on their own alignment, but a mask (an option group's, or a nullable
/// run's trailing its values), packed bools and `Str`/`Bytes` blobs end on
/// any byte. So this walks the writes in that order, keeping the alignment
/// the end is guaranteed to have, and adds up the worst padding each could
/// need. It starts from [`dynamic_region_alignment`], which the fixed region
/// is padded to.
///
/// Where the inodes cannot tell whether a write happens (whether a list
/// level has tiers of its own, say), it is counted as happening: an extra
/// word-aligned write can only add to the bound, never hide padding a later
/// write needs.
///
/// A message of scalars has none. So does `Vec<Vec<Vec<Option<u8>>>>`: its
/// tiers keep the structure 4-aligned, and a `u8` run and its mask need
/// none. `Vec<Option<Foo>>` of that has 3: its mask ends anywhere, and the
/// tiers after it pad back to a word.
pub const fn worst_padding<W: StructureWord>(inodes: &[Inode]) -> usize {
    let mut aligned = dynamic_region_alignment::<W>(inodes) as usize;
    let mut padding = 0;

    let mut i = 0;
    while i < inodes.len() {
        if let Some(leaves) = structure_write::<W>(inodes[i]) {
            padding += pad_from(aligned, W::SIZE);
            aligned = leaves;
        }
        i += 1;
    }

    let lanes = [16, 8, 4, 2, 1];
    let mut lane = 0;
    while lane < lanes.len() {
        let mut i = 0;
        while i < inodes.len() {
            if let Some(leaves) = payload_write(inodes[i], lanes[lane]) {
                padding += pad_from(aligned, lanes[lane]);
                aligned = leaves;
            }
            i += 1;
        }
        lane += 1;
    }
    padding
}

/// The most topology tiers the writers can open in a message laid out as
/// `inodes`, whatever its values: each one starts with a
/// `StructureWord::RESET` the plan's per-node words don't count, so the
/// plan's bound needs this many more structure words.
///
/// Every inode's structure is laid out once per message, however many values
/// it holds, so this is a sum over the table. An inode writes one tier per
/// list level above its leaf, less the one a bare run needs no tier for, plus
/// one when it is iterated under a list (`group_nest`):
///
/// - A primitive, fixed array, bool or group: `nesting - 1 + group_nest`, and
///   none as a scalar. A `BitVec` is already a `Bool` one nesting deeper.
/// - `Str`/`Bytes` are a list themselves: `nesting + group_nest`.
/// - A map is a list too: `nesting + group_nest`. A bare one (neither)
///   keeps its entry count in its slot and opens no tier; one iterated
///   under a list or map (`group_nest`, even at `nesting == 0`) opens a tier
///   of per-map entry counts. Its keys and values are inodes of their own,
///   and are always `group_nest`.
///
/// A tier with no nodes writes no reset, so this is an upper bound.
pub const fn topology_reset_count(inodes: &[Inode]) -> usize {
    let mut resets = 0;
    let mut i = 0;
    while i < inodes.len() {
        let inode = inodes[i];
        let nesting = inode.nesting() as usize;
        let group_nest = inode.group_nest() as usize;
        resets += match inode.kind {
            WireKind::Unknown => 0,
            WireKind::Str | WireKind::Bytes | WireKind::Map => nesting + group_nest,
            _ => {
                if nesting == 0 {
                    0
                } else {
                    nesting - 1 + group_nest
                }
            }
        };
        i += 1;
    }
    resets
}

/// The most padding from an end aligned to `aligned` up to `to`, both powers
/// of two.
const fn pad_from(aligned: usize, to: usize) -> usize {
    if aligned >= to { 0 } else { to - aligned }
}

/// Whether `inode` writes to the structure lane, and if so the alignment it
/// leaves the lane's end at: `4` after tier or offset words, `1` after an
/// option group's mask.
const fn structure_write<W: StructureWord>(inode: Inode) -> Option<usize> {
    let listed = inode.nesting() > 0 || inode.group_nest();
    match inode.kind {
        WireKind::Unknown => None,
        WireKind::Str | WireKind::Bytes | WireKind::Group => {
            if listed {
                Some(W::SIZE)
            } else {
                None
            }
        }
        WireKind::OptionGroup => {
            if listed {
                Some(1)
            } else {
                None
            }
        }
        WireKind::Map => Some(4),
        // A primitive's tiers: a list of lists, or a list under a list of
        // groups.
        _ => {
            if inode.nesting() >= 2 || inode.group_nest() {
                Some(W::SIZE)
            } else {
                None
            }
        }
    }
}

/// Whether `inode` writes to the payload lane aligned to `lane`, and if so
/// the alignment it leaves the lane's end at: its own after a plain run, `1`
/// after a mask, packed bools or a blob.
const fn payload_write(inode: Inode, lane: usize) -> Option<usize> {
    match inode.kind {
        WireKind::Unknown | WireKind::Group | WireKind::OptionGroup | WireKind::Map => None,
        WireKind::Str | WireKind::Bytes => {
            if lane == 1 {
                Some(1)
            } else {
                None
            }
        }
        _ => {
            // A scalar sits in the fixed region.
            if (inode.nesting() == 0 && !inode.group_nest()) || inode.alignment() != lane {
                return None;
            }
            // A nullable run's mask trails its values.
            let packed_or_masked = matches!(
                inode.kind,
                WireKind::Bool
                    | WireKind::OptionBool
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
            );
            if packed_or_masked {
                Some(1)
            } else {
                Some(lane)
            }
        }
    }
}

/// This takes in an Inode table that has had it's topology layout pass and
/// determines the length of that topology section.
///
/// The first part of all fixed messages is the relative pointers to either
/// the topology (in the case of nested vecs and groups) OR the relative pointer
/// to the slice of data the inode points to
///
/// # Precondition: call this after [`layout_topology`] has run
///
/// Reads the offsets `layout_topology` already assigned, using
/// [`Inode::group_nest`] (populated by that same pass) rather than
/// [`FIXED_ITEM_PENDING`] to tell a topology entry apart from a fixed
/// candidate — unlike the sentinel, `group_nest` is set once and never
/// touched again by the later passes, so (unlike an earlier version of
/// this function) it stays correct no matter how much of the rest of
/// [`layout_in_place`] has since run.
///
/// Every entry's contribution is "its own assigned offset + its own
/// pointer width", maxed across all entries:
///
/// - A primitive is topology-eligible exactly when `nesting() >= 1` (it's
///   itself a list) or `group_nest()` (some ancestor forces nested
///   treatment) — see [`KindMeta::group_nest`]. Its end is `+8` normally
///   or `+12` for an `Option` scalar's data pointer (see
///   `layout_topology`'s pass 1 for where those widths come from). Its own
///   `topology_offset`, when also assigned, is always 8 bytes earlier, so
///   it never needs separate consideration.
/// - `Str`/`Bytes` are always topology-eligible, regardless of nesting.
/// - `Group` only gets a `topology_offset` when `nesting() > 0`
///   (self-nested in a list) — that's knowable from the node alone, so it
///   gates inclusion directly.
/// - `OptionGroup` can *also* get a `topology_offset` purely from ancestor
///   context (nested inside an enclosing list without itself being one) —
///   a case `nesting()` alone can't see, but `Group`/`Map`/`OptionGroup`
///   were all given `group`/`group_nest` too (see [`KindMeta::group_nest`]),
///   so `nesting() > 0 || group_nest()` covers it exactly, same as for
///   primitives above.
/// - `Map` is always topology-eligible, unconditionally — a map's own
///   nesting is always treated as `nesting() + 1` for layout purposes (its
///   key/value pairs are inherently an out-of-line list even for a "bare"
///   map), so its `topology_offset` is always assigned.
pub const fn inode_topology_len<W: StructureWord>(inodes: &[Inode]) -> usize {
    let mut max_topology: u16 = 0;

    let mut i = 0;
    while i < inodes.len() {
        let inode = inodes[i];
        match inode.kind {
            WireKind::Unknown => {}
            WireKind::Bool
            | WireKind::I8
            | WireKind::I16
            | WireKind::I32
            | WireKind::I64
            | WireKind::U8
            | WireKind::U16
            | WireKind::U32
            | WireKind::U64
            | WireKind::F16
            | WireKind::F32
            | WireKind::F64
            | WireKind::Timestamp => {
                if inode.nesting() >= 1 || inode.group_nest() {
                    let end = inode.data_offset() + 2 * W::SIZE as u16;
                    if end > max_topology {
                        max_topology = end;
                    }
                }
            }
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
            | WireKind::OptionTimestamp => {
                if inode.nesting() >= 1 || inode.group_nest() {
                    let end = inode.data_offset() + 3 * W::SIZE as u16;
                    if end > max_topology {
                        max_topology = end;
                    }
                }
            }
            WireKind::Str | WireKind::Bytes => {
                let end = inode.data_offset() + 2 * W::SIZE as u16;
                if end > max_topology {
                    max_topology = end;
                }
            }
            WireKind::Group => {
                if inode.nesting() > 0 || inode.group_nest() {
                    let end = inode.topology_offset()
                        + group_slot_size::<W>(
                            WireKind::Group,
                            inode.nesting() as usize,
                            inode.group_nest(),
                        );
                    if end > max_topology {
                        max_topology = end;
                    }
                }
            }
            WireKind::OptionGroup => {
                if inode.nesting() > 0 || inode.group_nest() {
                    let end = inode.topology_offset()
                        + group_slot_size::<W>(
                            WireKind::OptionGroup,
                            inode.nesting() as usize,
                            inode.group_nest(),
                        );
                    if end > max_topology {
                        max_topology = end;
                    }
                }
            }
            // A Map is always topology-eligible — its effective nesting is
            // always node_nesting + 1 (layout_topology's `is_map`
            // adjustment), so its `topology_offset` is always assigned.
            WireKind::Map => {
                let end = inode.topology_offset()
                    + group_slot_size::<W>(
                        WireKind::Map,
                        inode.nesting() as usize,
                        inode.group_nest(),
                    );
                if end > max_topology {
                    max_topology = end;
                }
            }
        }
        i += 1;
    }

    max_topology as usize
}

/// This takes in an Inode table that has had it's topology layout AND it's
/// data layout and determines the length of both it's topology and data section.
///
/// This includes the topology above and the data bits. If we're writing a f64 after the
/// topology section this needs to account for the padding needed to pad the u32 at
/// 4 bytes to the f64 at 8 bytes.
///
/// This does NOT include the bit vector after this section that is encoded as
/// a BitVec<u8, Lsb0>.
///
/// # Precondition: call this after [`layout_fixed_data`] has run
///
/// Now that [`Inode::group_nest`] records exactly what would otherwise
/// need a depth-tracking tree walk to know ("is this primitive a genuine
/// fixed candidate, or a topology item whose own `nesting()` merely
/// happens to be 0 because some ancestor `Group`/`Map` carries the
/// nesting instead — e.g. a plain field inside `Vec<Group<...>>>`"), this
/// needs no traversal at all: a single flat pass over `inodes`, using
/// [`inode_topology_len`] as the floor and, for every non-option scalar
/// with `nesting() == 0 && !group_nest()` (a genuine non-nullable fixed
/// candidate), trusting `layout_fixed_data`'s own placement and reading
/// `data_offset() + size()` off it directly.
pub const fn inode_fixed_data_len<W: StructureWord>(inodes: &[Inode]) -> usize {
    let mut max_end = inode_topology_len::<W>(inodes) as u16;

    let mut i = 0;
    while i < inodes.len() {
        let inode = inodes[i];
        if matches!(
            inode.kind,
            WireKind::Bool
                | WireKind::I8
                | WireKind::I16
                | WireKind::I32
                | WireKind::I64
                | WireKind::U8
                | WireKind::U16
                | WireKind::U32
                | WireKind::U64
                | WireKind::F16
                | WireKind::F32
                | WireKind::F64
                | WireKind::Timestamp
        ) && inode.nesting() == 0
            && !inode.group_nest()
        {
            let end = inode.data_offset() + inode.size::<W>() as u16;
            if end > max_end {
                max_end = end;
            }
        }
        i += 1;
    }

    max_end as usize
}

/// This is the total length of the data with the bit vector at the end.
///
/// This includes everything above plus a BitVec<u8, Lsb0> with all the option flags.
///
/// # Precondition: call this after the full [`layout_in_place`] pipeline has run
///
/// Builds on [`inode_fixed_data_len`] as a floor, then adds the nullable
/// fixed region and its trailing bitmap. A genuine nullable fixed item is
/// exactly `nesting() == 0 && !group_nest()` on an `Option` scalar — the
/// same [`Inode::group_nest`]-based classification `inode_topology_len`
/// and `inode_fixed_data_len` use above, so an `Option` scalar that's
/// actually a topology item (ancestor-nested) is correctly excluded
/// without a second depth-tracking walk or an offset-comparison heuristic.
///
/// A plain `OptionGroup` ([`Inode::is_nullable_group`]) also consumes a
/// bit here — its own presence/absence, not inferred from its members —
/// but no fixed-region bytes (`Group`/`Map` never do), so it only ever
/// widens the bitmap, never `max_end`.
pub const fn inode_data_len<W: StructureWord>(inodes: &[Inode]) -> usize {
    let max_end = nullable_validity_start::<W>(inodes) as u16;
    let mut max_bit_index: i32 = -1;

    let mut i = 0;
    while i < inodes.len() {
        let inode = inodes[i];
        if inode.is_nullable_fixed() && inode.nesting() == 0 && !inode.group_nest() {
            let bit = inode.nullable_bit_index() as i32;
            if bit > max_bit_index {
                max_bit_index = bit;
            }
        } else if inode.is_nullable_group() {
            let bit = inode.nullable_bit_index() as i32;
            if bit > max_bit_index {
                max_bit_index = bit;
            }
        }
        i += 1;
    }

    let num_nullable = (max_bit_index + 1) as u16; // -1 -> 0 nullable fields
    let bitmap_bytes = num_nullable.div_ceil(8);

    let align = dynamic_region_alignment::<W>(inodes);
    let total = (max_end + bitmap_bytes).next_multiple_of(align);
    total as usize
}

/// Counts the number of top-level (depth-0) inodes in `inodes` — i.e. the
/// number of children the *implicit* root of this table has, reading
/// `inodes` as a flat prefix/Polish-notation encoding of a forest: a
/// `Group`/`OptionGroup` node is immediately followed by its
/// [`Inode::group_count`] children (each themselves possibly a subtree), and
/// a `Map` node is immediately followed by its 2 children (key, value) —
/// recursively, since a child can itself be a `Group`/`OptionGroup`/`Map`
/// with its own children inline.
///
/// A table with no `Group`/`OptionGroup`/`Map` nodes at all is just a flat
/// list of siblings, so every entry is top-level and this returns
/// `inodes.len()`. Once the first entry is a container, though, everything
/// that follows up to its subtree's end is a *descendant*, not a sibling —
/// e.g. a lone `Group { count: 2 }` followed by its 2 fields is 3 inodes
/// total but only 1 child (the group itself; its fields aren't top-level).
///
/// Mirrors the same `(remaining_children stack, depth)` traversal
/// [`layout_topology`] and [`layout_nullable_section`] use to walk this
/// same flat encoding, but only needs the remaining-child counts (no
/// per-frame extras like `is_map_child` or `parent_repeats`) since all it
/// tracks is "am I currently inside someone's subtree" (`stack_len > 0`).
pub const fn count_children(inodes: &[Inode]) -> usize {
    let n = inodes.len();
    let mut count = 0usize;
    let mut stack: [usize; MAX_GROUP_DEPTH] = [0; MAX_GROUP_DEPTH];
    let mut stack_len: usize = 0;

    let mut idx = 0;
    while idx < n {
        if stack_len == 0 {
            count += 1;
        }

        let num_children = match inodes[idx].kind {
            WireKind::Group | WireKind::OptionGroup => inodes[idx].group_count() as usize,
            WireKind::Map => 2,
            _ => 0,
        };
        if stack_len > 0 {
            stack[stack_len - 1] -= 1;
        }
        if num_children > 0 {
            if stack_len >= MAX_GROUP_DEPTH {
                panic!("count_children: group nesting exceeds MAX_GROUP_DEPTH (64)");
            }
            stack[stack_len] = num_children;
            stack_len += 1;
        }
        while stack_len > 0 && stack[stack_len - 1] == 0 {
            stack_len -= 1;
        }

        idx += 1;
    }

    count
}

/// Assigns every `data_offset` / `topology_offset` / `nullable_bit_index` in
/// `inodes` in place. Returns the total fixed-region byte count
/// (`bitmap_offset + bitmap_bytes`).
///
/// This is the const-evaluable core of `InodeBuilder::build_with_fixed_size`
/// — same algorithm, ported to `while` index loops with zero heap scratch so
/// it can run both at const-eval time (from a `static` initializer building
/// a `Layout::INODES` table) and at runtime (from the existing
/// `OnceLock` path for generic structs).
///
/// The algorithm, in two passes:
///
/// 1. Walk `inodes` once, tracking group/list nesting depth with a
///    fixed-size stack. Any inode that needs a pointer into the dynamic
///    region (nested inside a list, or a `Str`/`Bytes`/`Group`/`Map` node)
///    gets its `topology_offset`/`data_offset` assigned immediately, in
///    strictly increasing `topology_byte_count` order. Any primitive scalar
///    that instead belongs in the *fixed* region (nesting == 0, depth == 0)
///    is left marked [`FIXED_ITEM_PENDING`] rather than collected into a
///    side list — there's nowhere else to leave a "come back to this" note
///    without a `Vec`, and this field is otherwise unused at this point.
/// 2. Ten more passes over the full `inodes` slice — non-nullable then
///    nullable, each descending alignment 16 → 8 → 4 → 2 → 1 — each one
///    scanning for `FIXED_ITEM_PENDING` items whose `alignment()` and
///    nullability match that pass, assigning them offsets in ascending
///    index order. This reproduces the old bucket-Vec push order exactly
///    (ascending index within an ascending-then-nullable alignment order),
///    so offsets come out byte-identical. Because nullable offsets are
///    necessarily assigned in that same strictly increasing order, the
///    nullable pass also assigns `nullable_bit_index` inline via a simple
///    counter — no `sort_by_key` needed, since bucket-traversal order and
///    ascending-`data_offset` order are the same order here.
pub const fn layout_in_place<W: StructureWord>(inodes: &mut [Inode]) -> usize {
    let topology_byte_count = layout_topology::<W>(inodes);
    let current_offset = layout_fixed_data::<W>(inodes, topology_byte_count);
    let bitmap_bytes = layout_nullable_section(inodes);

    // Pad to the topology requirement: the dynamic region that follows
    // this fixed region + bitmap must start on a boundary matching
    // whatever gets written there first (see `dynamic_region_alignment`).
    let align = dynamic_region_alignment::<W>(inodes);
    (current_offset + bitmap_bytes as u16).next_multiple_of(align) as usize
}

/// Pass 1: assign topology pointers, and mark every fixed-region candidate
/// with [`FIXED_ITEM_PENDING`] in its own `data_offset`. Returns the total
/// topology-section byte count, i.e. the offset the fixed region starts at.
/// Bytes the fixed slot of a group-shaped node (`Group`, `OptionGroup`,
/// `Map`) occupies.
///
/// A `List<Group<..>>` stores nothing but its element count: the group's
/// fields each carry their own data pointer, and the count is the only
/// thing a reader cannot recover from them — so the slot is a single
/// 4-byte word. One list level further out (`List<List<Group<..>>>`) the
/// inner lists' boundaries need a real prefix array, so the slot goes back
/// to the usual 8-byte `(start, end)` pair addressing it.
///
/// A `List<Option<Group<..>>>` always takes 8: at one level that is a
/// relative pointer to the validity bitmap plus the element count, and
/// above it the `(start, end)` pair, with the bitmap trailing the arrays.
///
/// A `Map` is a `List<Group<key, value>>`, so it sizes like a group one
/// list level deeper than its own `nesting` — a bare map included, which
/// is why a bare map owns a slot at all.
///
/// `group_nest` is the node's own [`Inode::group_nest`] flag: true when
/// some ancestor group/map already contributes a list level, which is what
/// turns a `nesting == 1` group into a two-level one.
pub const fn group_slot_size<W: StructureWord>(
    kind: WireKind,
    nesting: usize,
    group_nest: bool,
) -> u16 {
    let list_depth = match kind {
        // A map's key/value pairs are inherently an out-of-line list.
        WireKind::Map => nesting + 1,
        _ => nesting,
    };
    let two_or_more = list_depth >= 2 || (list_depth >= 1 && group_nest);
    match kind {
        // The bitmap pointer costs a word no matter the depth.
        WireKind::OptionGroup => W::SIZE as u16 * 2,
        _ if two_or_more => W::SIZE as u16 * 2,
        _ => W::SIZE as u16,
    }
}

pub const fn layout_topology<W: StructureWord>(inodes: &mut [Inode]) -> u16 {
    let n = inodes.len();
    if n > MAX_INODES {
        panic!("layout_in_place: schema has more than MAX_INODES (1024) inodes");
    }

    // ---- Pass 1: topology pointers; mark fixed items pending. ----
    let mut topology_byte_count: u16 = 0;
    // (remaining_children, parent_depth)
    let mut stack: [(usize, usize); MAX_GROUP_DEPTH] = [(0, 0); MAX_GROUP_DEPTH];
    let mut stack_len: usize = 0;
    let mut current_depth: usize = 0;

    let mut idx = 0;
    while idx < n {
        let node_nesting = inodes[idx].nesting() as usize;
        // A `Map` behaves exactly like a `Group` whose own `nesting` is one
        // more than its declared value: its key/value pairs are inherently
        // an out-of-line list, even for a "bare" (`nesting == 0`) map — see
        // `layout_topology`'s `Map` arm below and 3.7 (wiki). This `+ 1` is
        // purely a layout-depth bookkeeping detail, local to this pass —
        // the stored `nesting` field on the `Map` inode and on its
        // children is never mutated to carry it.
        let is_map = matches!(inodes[idx].kind, WireKind::Map);
        let depth = current_depth + node_nesting + if is_map { 1 } else { 0 };
        match (inodes[idx].kind, &mut inodes[idx].kind_meta) {
            (WireKind::Unknown, _) => {}
            (
                WireKind::Bool
                | WireKind::I8
                | WireKind::I16
                | WireKind::I32
                | WireKind::I64
                | WireKind::U8
                | WireKind::U16
                | WireKind::U32
                | WireKind::U64
                | WireKind::F16
                | WireKind::F32
                | WireKind::F64
                | WireKind::Timestamp,
                KindMeta::Primitive {
                    data_offset,
                    topology_offset,
                    group,
                    group_nest,
                    ..
                },
            ) => {
                *group = stack_len > 0;
                *group_nest = current_depth > 0;
                if node_nesting > 1 || (node_nesting == 1 && depth > 1) {
                    *topology_offset = topology_byte_count;
                    topology_byte_count += W::SIZE as u16 * 2; // start + len ptr
                    *data_offset = topology_byte_count;
                    topology_byte_count += W::SIZE as u16 * 2; // start + len ptr
                } else if node_nesting == 1 || (node_nesting == 0 && depth > 0) {
                    *data_offset = topology_byte_count;
                    topology_byte_count += W::SIZE as u16 * 2; // start + len ptr
                } else {
                    *data_offset = FIXED_ITEM_PENDING;
                }
            }
            (
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
                | WireKind::OptionTimestamp,
                KindMeta::Primitive {
                    data_offset,
                    topology_offset,
                    group,
                    group_nest,
                    ..
                },
            ) => {
                *group = stack_len > 0;
                *group_nest = current_depth > 0;
                if node_nesting > 1 || (node_nesting == 1 && depth > 1) {
                    *topology_offset = topology_byte_count;
                    topology_byte_count += W::SIZE as u16 * 2; // start + len ptr
                    *data_offset = topology_byte_count;
                    topology_byte_count += W::SIZE as u16 * 3; // start + len ptr
                } else if node_nesting == 1 || (node_nesting == 0 && depth > 0) {
                    *data_offset = topology_byte_count;
                    topology_byte_count += W::SIZE as u16 * 3; // start + len ptr
                } else {
                    *data_offset = FIXED_ITEM_PENDING;
                }
            }
            (
                WireKind::Bool
                | WireKind::I8
                | WireKind::I16
                | WireKind::I32
                | WireKind::I64
                | WireKind::U8
                | WireKind::U16
                | WireKind::U32
                | WireKind::U64
                | WireKind::F16
                | WireKind::F32
                | WireKind::F64
                | WireKind::Timestamp
                | WireKind::OptionBool
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
                | WireKind::Str
                | WireKind::Bytes,
                KindMeta::Group { .. } | KindMeta::Map { .. },
            ) => {
                panic!("layout_in_place: primitive wire kind paired with Group/Map kind_meta");
            }
            (
                WireKind::Str | WireKind::Bytes,
                KindMeta::Primitive {
                    data_offset,
                    topology_offset,
                    group,
                    group_nest,
                    ..
                },
            ) => {
                *group = stack_len > 0;
                *group_nest = current_depth > 0;
                if depth > 0 {
                    // When nested inside a GroupList, Str/Bytes needs a topology
                    // pointer (for the NestedListTopology that tracks per-element
                    // string offsets) in addition to its own data pointer.
                    *topology_offset = topology_byte_count;
                    topology_byte_count += W::SIZE as u16 * 2; // topology start + end ptr
                }
                *data_offset = topology_byte_count;
                topology_byte_count += W::SIZE as u16 * 2; // data start + len ptr
            }
            (
                WireKind::Group,
                KindMeta::Group {
                    topology_offset,
                    group,
                    group_nest,
                    ..
                },
            ) => {
                *group = stack_len > 0;
                *group_nest = current_depth > 0;
                // A listed group owns a slot, and so does one repeated by an
                // ancestor (a field of a listed group, a map's key or
                // value): at `nesting == 0` that slot is a single word, the
                // number of instances. Its parent's slot can't stand in for
                // it — a nested map's or listed group's slot is a topology
                // pointer, not a count. Same rule as `OptionGroup` below.
                if node_nesting > 0 || current_depth > 0 {
                    *topology_offset = topology_byte_count;
                    topology_byte_count +=
                        group_slot_size::<W>(WireKind::Group, node_nesting, current_depth > 0);
                }
            }
            (
                WireKind::OptionGroup,
                KindMeta::Group {
                    topology_offset,
                    group,
                    group_nest,
                    ..
                },
            ) => {
                *group = stack_len > 0;
                *group_nest = current_depth > 0;
                // If we're an option group inside a list we need to track our
                // topology for our nullability. Either way the slot is 8
                // bytes: at one list level that is `(validity pointer,
                // element count)`, and deeper it is the `(start, end)` pair
                // addressing the prefix arrays, with the validity bitmap
                // trailing them — see [`group_slot_size`].
                if node_nesting >= 1 || current_depth > 0 {
                    *topology_offset = topology_byte_count;
                    topology_byte_count += group_slot_size::<W>(
                        WireKind::OptionGroup,
                        node_nesting,
                        current_depth > 0,
                    );
                }
                // Otherwise (a plain, non-list, non-ancestor-nested
                // OptionGroup) its presence/absence gets a real bit in the
                // shared nullable bitmap instead — see layout_nullable_section.
            }
            (
                WireKind::Map,
                KindMeta::Map {
                    topology_offset,
                    group,
                    group_nest,
                    ..
                },
            ) => {
                *group = stack_len > 0;
                *group_nest = current_depth > 0;
                // A map is a `List<Group<key, value>>`, so *every* map owns a
                // slot — a bare one included, where it holds the pair count
                // exactly as a `List<Group<..>>` holds its element count.
                *topology_offset = topology_byte_count;
                topology_byte_count +=
                    group_slot_size::<W>(WireKind::Map, node_nesting, current_depth > 0);
            }
            (
                WireKind::Group | WireKind::OptionGroup,
                KindMeta::Primitive { .. } | KindMeta::Map { .. },
            ) => {
                panic!(
                    "layout_in_place: Group/OptionGroup wire kind paired with non-Group kind_meta"
                );
            }
            (WireKind::Map, KindMeta::Primitive { .. } | KindMeta::Group { .. }) => {
                panic!("layout_in_place: Map wire kind paired with non-Map kind_meta");
            }
        }

        // Tree traversal logic to track parent depth accurately.
        let num_children = match inodes[idx].kind {
            WireKind::Group | WireKind::OptionGroup => inodes[idx].group_count() as usize,
            WireKind::Map => 2,
            _ => 0,
        };
        if stack_len > 0 {
            stack[stack_len - 1].0 -= 1;
        }

        if num_children > 0 {
            if stack_len >= MAX_GROUP_DEPTH {
                panic!("layout_in_place: group nesting exceeds MAX_GROUP_DEPTH (64)");
            }
            stack[stack_len] = (num_children, current_depth);
            stack_len += 1;
            current_depth = depth;
        }

        while stack_len > 0 && stack[stack_len - 1].0 == 0 {
            current_depth = stack[stack_len - 1].1;
            stack_len -= 1;
        }

        idx += 1;
    }

    topology_byte_count
}

/// Passes 2..7: non-nullable fixed items only, alignment 16 -> 1. Takes the
/// topology-section byte count from [`layout_topology`] as the starting
/// offset, and returns the offset immediately past the non-nullable fixed
/// region (where [`layout_nullable_section`]'s own fixed-item pass starts).
///
/// Deliberately does not touch nullability at all — a nullable fixed item
/// is left `FIXED_ITEM_PENDING` here, for [`layout_nullable_section`] to
/// place in its own separate pass.
pub const fn layout_fixed_data<W: StructureWord>(
    inodes: &mut [Inode],
    topology_byte_count: u16,
) -> u16 {
    let n = inodes.len();
    let mut current_offset = topology_byte_count;
    let aligns: [u16; 5] = [16, 8, 4, 2, 1];

    let mut b = 0;
    while b < 5 {
        let align = aligns[b];
        let mut first = true;
        let mut i = 0;
        while i < n {
            let inode = inodes[i];
            if inode.data_offset() == FIXED_ITEM_PENDING && inode.alignment() as u16 == align {
                if first {
                    current_offset = current_offset.next_multiple_of(align);
                    first = false;
                }
                let total_size = inode.size::<W>() as u16;
                if let KindMeta::Primitive { data_offset, .. } = &mut inodes[i].kind_meta {
                    *data_offset = current_offset;
                }
                current_offset += total_size;
            }
            i += 1;
        }
        b += 1;
    }

    current_offset
}

/// Passes 8..13: nullable fixed items, alignment 16 -> 1, assigning
/// `nullable_bit_index` inline as they're placed in ascending `data_offset`
/// order (no `sort_by_key` needed) — then group-presence bits, then the
/// trailing bitmap. Takes the offset [`layout_fixed_data`] left off at, and
/// returns the total fixed-region byte count (`bitmap_offset +
/// bitmap_bytes`).
///
/// Deliberately a whole separate five-tier pass from [`layout_fixed_data`],
/// not merged with it — `layout_fixed_data` lays out non-nullable fixed
/// data only, full stop. This isn't the fastest possible layout (a
/// nullable field can't share a tier scan with a non-nullable one of the
/// same alignment), but nobody pays for that at runtime: the derive bakes
/// the computed offsets in as compile-time constants for the vast majority
/// of structs, so this only runs at all for the generic/dynamic fallback
/// path.
///
/// A plain (non-list, non-ancestor-nested) `OptionGroup`'s own
/// presence/absence ([`Inode::is_nullable_group`]) also gets a real bit
/// here, continuing the same counter after every nullable primitive —
/// not inferred from whether its members look absent, since a group with
/// zero nullable members (or zero members at all) still needs to know
/// whether it's there.
pub const fn layout_nullable_section(inodes: &mut [Inode]) -> usize {
    let n = inodes.len();
    let mut i = 0;
    let mut bit_index: u16 = 0;

    // Mirrors `layout_topology`'s own stack traversal, but only needs one
    // bit of context per open frame: does the *immediately enclosing*
    // Group/OptionGroup/Map itself cause per-instance repetition (its own
    // `nesting() > 0`, or it's a `Map` — bare Maps are always inherently a
    // list of key/value pairs)? That's different from `group_nest()`
    // (`current_depth > 0`), which is true for *any* ancestor nesting, not
    // just the direct parent.
    //
    // A nullable primitive whose *direct* parent repeats already carries
    // its own per-instance presence inside its own topology entry (the
    // 12-byte start/count/validity form `layout_topology` gives every
    // group_nest Option primitive) — it doesn't also need a bit in the
    // shared fixed-region bitmap, so it's excluded from this counter
    // entirely (its `nullable_bit_index` stays the default 0, unused).
    // A nullable primitive nested only through a purely *structural*
    // (non-repeating) ancestor group, though, occurs exactly once per
    // enclosing instance — same as a plain top-level nullable field — so
    // it still shares the ordinary incrementing counter.
    let mut stack: [(usize, bool); MAX_GROUP_DEPTH] = [(0, false); MAX_GROUP_DEPTH];
    let mut stack_len: usize = 0;
    let mut parent_repeats = false;

    while i < n {
        let inode = inodes[i];
        let node_nesting = inode.nesting();
        let is_map = matches!(inode.kind, WireKind::Map);

        let excluded_by_direct_repetition = inode.group_nest() && parent_repeats;
        if (inode.is_nullable_fixed() && !excluded_by_direct_repetition)
            || inode.is_nullable_group()
        {
            if let KindMeta::Primitive {
                nullable_bit_index, ..
            } = &mut inodes[i].kind_meta
            {
                *nullable_bit_index = bit_index;
            }
            if let KindMeta::Group {
                nullable_bit_index, ..
            } = &mut inodes[i].kind_meta
            {
                *nullable_bit_index = bit_index;
            }
            bit_index += 1;
        }

        let num_children = match inodes[i].kind {
            WireKind::Group | WireKind::OptionGroup => inodes[i].group_count() as usize,
            WireKind::Map => 2,
            _ => 0,
        };
        if stack_len > 0 {
            stack[stack_len - 1].0 -= 1;
        }
        if num_children > 0 {
            if stack_len >= MAX_GROUP_DEPTH {
                panic!("layout_nullable_section: group nesting exceeds MAX_GROUP_DEPTH (64)");
            }
            // Cumulative, like `group_nest()`'s own `current_depth > 0`
            // (`layout_topology`'s pass 1): once *any* ancestor repeats,
            // every descendant primitive already carries its own
            // per-instance presence in its topology entry, no matter how
            // many non-repeating structural groups sit between it and that
            // ancestor — a nullable primitive two `Option<Struct>` frames
            // under a `Vec<Struct>` is still `group_nest()`, and needs no
            // bit here either. `parent_repeats` has to fold forward the
            // same way `current_depth` does, or this pass under-excludes
            // primitives whose repeating ancestor isn't their *direct*
            // parent — see `src/wire/tests/group_option_nesting.rs`'s
            // `LIST_GROUP_OPTION_GROUP_PRESENT` et al., whose `Option<i8>`
            // sits under a non-repeating `Option<Struct>` under a
            // repeating list.
            let this_repeats = parent_repeats || node_nesting > 0 || is_map;
            stack[stack_len] = (num_children, this_repeats);
            stack_len += 1;
            parent_repeats = this_repeats;
        }
        while stack_len > 0 && stack[stack_len - 1].0 == 0 {
            stack_len -= 1;
            parent_repeats = if stack_len > 0 {
                stack[stack_len - 1].1
            } else {
                false
            };
        }

        i += 1;
    }

    let num_nullable = bit_index as usize;
    let bitmap_bytes: u16 = if num_nullable > 0 {
        ((num_nullable + 7) / 8) as u16 // ceil division
    } else {
        0
    };

    bitmap_bytes as usize
}

// This is tested in src/wire/tests. Each lays out inodes and verifies them against static ones.
