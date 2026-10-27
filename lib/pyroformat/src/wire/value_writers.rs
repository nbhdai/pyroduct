pub mod fixed;
pub(crate) mod group;
pub(crate) mod interface;
pub(crate) mod map;
pub(crate) mod nested;
pub(crate) mod nested_basic;
pub(crate) mod nested_fixed;
pub(crate) mod nested_option_fixed;
pub(crate) mod nested_option_slices;
pub(crate) mod nested_slices;
pub(crate) mod primitive;
pub(crate) mod scalar;

use crate::buf::PyroSink;
use crate::wire::{
    IoType,
    value_writers::{
        group::{Group, NestedGroup, NestedOptionGroup, OptionGroup},
        interface::{Padding, SplitSink},
        nested::Nested,
        scalar::Scalar,
    },
};

use crate::errors::WireError;
use crate::schema::PyroType;
use crate::value::PyroValue;
use map::Map;

use pyro_schema::{
    Inode, InodeMeta, PyroSchema, SchemaParts, StructureWord, WireKind, Word, inode_data_len,
};

#[derive(Debug, Clone)]
pub enum RootWriter<'a, W: StructureWord> {
    Map(Map<'a, W>),
    Group(Group<'a, W>),
    OptionGroup(OptionGroup<'a, W>),
    Scalar(Scalar<'a>),
    Nested(Nested<'a, W>),
    GroupNested(NestedGroup<'a, W>),
    OptionGroupNested(NestedOptionGroup<'a, W>),
}

#[derive(Debug, Clone)]
pub enum NestedWriter<'a, W: StructureWord> {
    Map(Box<map::Map<'a, W>>),
    Nested(Nested<'a, W>),
    GroupNested(NestedGroup<'a, W>),
    OptionGroupNested(NestedOptionGroup<'a, W>),
}

/// Builds the *entire* `match inode.kind { .. }` body of [`RootWriter::new`]
/// as a single macro expansion — a bare macro invocation can't stand in for
/// a handful of arms inside someone else's `match`, so the numeric arms
/// (`I8` .. `F64`, plus their `Option`/fixed-array variants) are spliced in
/// alongside the explicit ones here rather than the other way around.
///
/// `$n` is the inode, `$k` its fixed-size array length (`0` for none) and
/// `$nesting` the `Vec`s around the value. A nesting-0 value is a
/// [`Scalar`], written inline; anything deeper is [`Nested`] at
/// `depth = nesting - 1`, the outermost `Vec` being the slot's own
/// `(start, count)`. `Str`/`Bytes` own an extra byte-boundary level, so
/// they are `Nested` at `depth = nesting` even bare.
macro_rules! root_writer_new_body {
    ($inode:expr, $n:ident, $k:ident, $nesting:ident, [$($Kind:ident),+ $(,)?]) => {
        paste::paste! {
            match $inode.kind {
                WireKind::Str => RootWriter::Nested(Nested::Str(nested_basic::StrWriter::new($nesting, $n))),
                WireKind::Bytes => RootWriter::Nested(Nested::Bytes(nested_basic::BytesWriter::new($nesting, $n))),

                WireKind::Bool if $k > 0 => {
                    if $nesting == 0 {
                        RootWriter::Scalar(Scalar::BoolFixed(fixed::BoolFixedWriter::new($k, $n)))
                    } else {
                        RootWriter::Nested(Nested::BoolFixed(nested_fixed::BoolFixedWriter::new($k, $nesting - 1, $n)))
                    }
                }
                WireKind::OptionBool if $k > 0 => {
                    if $nesting == 0 {
                        RootWriter::Scalar(Scalar::BoolFixedOption(fixed::BoolFixedOptionWriter::new($k, $n)))
                    } else {
                        RootWriter::Nested(Nested::BoolOptionFixed(
                            nested_option_fixed::BoolOptionFixedWriter::new($k, $nesting - 1, $n),
                        ))
                    }
                }
                WireKind::Bool => {
                    if $nesting == 0 {
                        RootWriter::Scalar(Scalar::Bool(primitive::BoolWriter::new($n)))
                    } else {
                        RootWriter::Nested(Nested::BoolSlice(nested_slices::BoolSliceWriter::new($nesting - 1, $n)))
                    }
                }
                WireKind::OptionBool => {
                    if $nesting == 0 {
                        RootWriter::Scalar(Scalar::BoolOption(primitive::OptionBoolWriter::new($n)))
                    } else {
                        RootWriter::Nested(Nested::BoolOptionSlice(
                            nested_option_slices::BoolOptionSliceWriter::new($nesting - 1, $n),
                        ))
                    }
                }

                $(
                    WireKind::$Kind => {
                        if $k > 0 {
                            if $nesting == 0 {
                                RootWriter::Scalar(Scalar::[<$Kind Fixed>](fixed::[<$Kind FixedWriter>]::new($k, $n)))
                            } else {
                                RootWriter::Nested(Nested::[<$Kind Fixed>](
                                    nested_fixed::[<$Kind FixedWriter>]::new($k, $nesting - 1, $n),
                                ))
                            }
                        } else if $nesting == 0 {
                            RootWriter::Scalar(Scalar::$Kind(primitive::[<$Kind Writer>]::new($n)))
                        } else {
                            RootWriter::Nested(Nested::[<$Kind Slice>](
                                nested_slices::[<$Kind SliceWriter>]::new($nesting - 1, $n),
                            ))
                        }
                    }
                    WireKind::[<Option $Kind>] => {
                        if $k > 0 {
                            if $nesting == 0 {
                                RootWriter::Scalar(Scalar::[<$Kind FixedOption>](
                                    fixed::[<$Kind FixedOptionWriter>]::new($k, $n),
                                ))
                            } else {
                                RootWriter::Nested(Nested::[<$Kind OptionFixed>](
                                    nested_option_fixed::[<$Kind OptionFixedWriter>]::new($k, $nesting - 1, $n),
                                ))
                            }
                        } else if $nesting == 0 {
                            RootWriter::Scalar(Scalar::[<$Kind Option>](primitive::[<Option $Kind Writer>]::new($n)))
                        } else {
                            RootWriter::Nested(Nested::[<$Kind OptionSlice>](
                                nested_option_slices::[<$Kind OptionSliceWriter>]::new($nesting - 1, $n),
                            ))
                        }
                    }
                )+

                WireKind::Group | WireKind::OptionGroup | WireKind::Map | WireKind::Unknown => {
                    unreachable!(
                        "RootWriter::new called with a non-leaf inode kind: {:?}",
                        $inode.kind
                    )
                }
            }
        }
    };
}

/// The `NestedWriter`-context twin of [`root_writer_new_body`]: always
/// inside a repeated container, so `depth = nesting` (one more than at the
/// root: the container's own level replaces the slot's) and there is no
/// `Scalar` case at all. `Str`/`Bytes` are `nesting + 1` for their byte
/// level.
macro_rules! nested_writer_new_body {
    ($inode:expr, $n:ident, $k:ident, $nesting:ident, [$($Kind:ident),+ $(,)?]) => {
        paste::paste! {
            match $inode.kind {
                WireKind::Str => NestedWriter::Nested(Nested::Str(nested_basic::StrWriter::new($nesting + 1, $n))),
                WireKind::Bytes => NestedWriter::Nested(Nested::Bytes(nested_basic::BytesWriter::new($nesting + 1, $n))),

                WireKind::Bool if $k > 0 => NestedWriter::Nested(Nested::BoolFixed(
                    nested_fixed::BoolFixedWriter::new($k, $nesting, $n),
                )),
                WireKind::Bool => NestedWriter::Nested(Nested::BoolSlice(
                    nested_slices::BoolSliceWriter::new($nesting, $n),
                )),
                WireKind::OptionBool if $k > 0 => NestedWriter::Nested(Nested::BoolOptionFixed(
                    nested_option_fixed::BoolOptionFixedWriter::new($k, $nesting, $n),
                )),
                WireKind::OptionBool => NestedWriter::Nested(Nested::BoolOptionSlice(
                    nested_option_slices::BoolOptionSliceWriter::new($nesting, $n),
                )),

                $(
                    WireKind::$Kind => {
                        if $k > 0 {
                            NestedWriter::Nested(Nested::[<$Kind Fixed>](
                                nested_fixed::[<$Kind FixedWriter>]::new($k, $nesting, $n),
                            ))
                        } else {
                            NestedWriter::Nested(Nested::[<$Kind Slice>](
                                nested_slices::[<$Kind SliceWriter>]::new($nesting, $n),
                            ))
                        }
                    }
                    WireKind::[<Option $Kind>] => {
                        if $k > 0 {
                            NestedWriter::Nested(Nested::[<$Kind OptionFixed>](
                                nested_option_fixed::[<$Kind OptionFixedWriter>]::new($k, $nesting, $n),
                            ))
                        } else {
                            NestedWriter::Nested(Nested::[<$Kind OptionSlice>](
                                nested_option_slices::[<$Kind OptionSliceWriter>]::new($nesting, $n),
                            ))
                        }
                    }
                )+

                WireKind::Group | WireKind::OptionGroup | WireKind::Map | WireKind::Unknown => {
                    unreachable!(
                        "NestedWriter::new called with a non-leaf inode kind: {:?}",
                        $inode.kind
                    )
                }
            }
        }
    };
}

/// `$e` on whichever writer `$self` holds, as `$w`.
macro_rules! root_dispatch {
    ($self:expr, $w:ident => $e:expr) => {
        match $self {
            RootWriter::Map($w) => $e,
            RootWriter::Group($w) => $e,
            RootWriter::OptionGroup($w) => $e,
            RootWriter::Scalar($w) => $e,
            RootWriter::Nested($w) => $e,
            RootWriter::GroupNested($w) => $e,
            RootWriter::OptionGroupNested($w) => $e,
        }
    };
}

/// [`root_dispatch`] for the writers with variable-region data; a
/// [`Scalar`] writes inline and has none, so it gets `$scalar`.
macro_rules! root_dispatch_data {
    ($self:expr, $scalar:expr, $w:ident => $e:expr) => {
        match $self {
            RootWriter::Scalar(_) => $scalar,
            RootWriter::Map($w) => $e,
            RootWriter::Group($w) => $e,
            RootWriter::OptionGroup($w) => $e,
            RootWriter::Nested($w) => $e,
            RootWriter::GroupNested($w) => $e,
            RootWriter::OptionGroupNested($w) => $e,
        }
    };
}

/// `$e` on whichever writer `$self` holds, as `$w`.
macro_rules! nested_dispatch {
    ($self:expr, $w:ident => $e:expr) => {
        match $self {
            NestedWriter::Map($w) => $e,
            NestedWriter::Nested($w) => $e,
            NestedWriter::GroupNested($w) => $e,
            NestedWriter::OptionGroupNested($w) => $e,
        }
    };
}

impl<'a, W: StructureWord> RootWriter<'a, W> {
    /// Build the writer for *leaf* inode `n` — never `Group`, `OptionGroup`
    /// or `Map`, which need their already-built children and are assembled
    /// directly by [`ValueWriter::new`] instead (mirrors
    /// `wire::value_readers::RowReader::new`).
    ///
    /// Only valid for a leaf whose immediate context does *not* force nested
    /// treatment (`!inode.group_nest()`): a genuine top-level schema field,
    /// or a flat (non-repeated) field of a flat `Group`/`OptionGroup` — both
    /// lay out identically, just at different offsets, which the inode
    /// already carries. A `group_nest()`'d leaf needs [`NestedWriter::new`]
    /// instead.
    pub fn new(n: usize, inode: &Inode) -> Self {
        debug_assert!(
            !inode.group_nest(),
            "RootWriter::new is for flat/root leaves; a group_nest()'d inode needs NestedWriter::new"
        );
        let nesting = inode.nesting() as usize;
        let k = inode.kind_meta.fixed_size() as usize;
        root_writer_new_body!(
            inode,
            n,
            k,
            nesting,
            [
                I8, I16, I32, I64, U8, U16, U32, U64, F16, F32, F64, Timestamp
            ]
        )
    }

    pub fn datatype(&self) -> PyroType {
        // `Map` also has an inherent `datatype(&self) -> &PyroType` (the raw
        // key/value type, ignoring any wrapping depth); the `IoType` one
        // wraps it in `List`s per `Map::depth()`, so it must be called
        // explicitly rather than through method lookup.
        root_dispatch!(self, w => IoType::datatype(w))
    }

    pub fn ingest(&mut self, item: &'a PyroValue) -> Result<(), WireError> {
        root_dispatch!(self, w => w.ingest(item))
    }

    pub fn variable_length(&self) -> usize {
        root_dispatch_data!(self, 0, w => w.variable_length())
    }

    pub fn padding_structure(&self, padding: &mut Padding) {
        root_dispatch_data!(self, (), w => w.padding_structure(padding))
    }

    pub fn padding_data(&self, pass: usize, padding: &mut Padding) {
        root_dispatch_data!(self, (), w => w.padding_data(pass, padding))
    }

    /// A [`Scalar`] writes itself inline here; everything else writes its
    /// own headers and topology.
    pub fn write_structure(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        root_dispatch!(self, w => w.write_structure(buffer))
    }

    pub fn write_16(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        root_dispatch_data!(self, Ok(()), w => w.write_16(buffer))
    }

    pub fn write_8(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        root_dispatch_data!(self, Ok(()), w => w.write_8(buffer))
    }

    pub fn write_4(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        root_dispatch_data!(self, Ok(()), w => w.write_4(buffer))
    }

    pub fn write_2(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        root_dispatch_data!(self, Ok(()), w => w.write_2(buffer))
    }

    pub fn write_1(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        root_dispatch_data!(self, Ok(()), w => w.write_1(buffer))
    }

    pub fn clear(self) -> RootWriter<'static, W> {
        match self {
            RootWriter::Map(v) => RootWriter::Map(v.clear()),
            RootWriter::Group(v) => RootWriter::Group(v.clear()),
            RootWriter::OptionGroup(v) => RootWriter::OptionGroup(v.clear()),
            RootWriter::Scalar(v) => RootWriter::Scalar(v.clear()),
            RootWriter::Nested(v) => RootWriter::Nested(v.clear()),
            RootWriter::GroupNested(v) => RootWriter::GroupNested(v.clear()),
            RootWriter::OptionGroupNested(v) => RootWriter::OptionGroupNested(v.clear()),
        }
    }
}

impl<'a, W: StructureWord> NestedWriter<'a, W> {
    /// Build the writer for *leaf* inode `n` whose immediate context forces
    /// nested treatment (`inode.group_nest() == true`): a field living
    /// directly on a repeated `Group`/`OptionGroup`, or a `Map` key/value.
    /// Never `Group`, `OptionGroup` or `Map` themselves — those need their
    /// already-built children and are assembled directly by
    /// [`ValueWriter::new`]. A leaf with `!inode.group_nest()` needs
    /// [`RootWriter::new`] instead.
    pub fn new(n: usize, inode: &Inode) -> Self {
        debug_assert!(
            inode.group_nest(),
            "NestedWriter::new is for group_nest()'d leaves; a flat/root inode needs RootWriter::new"
        );
        let nesting = inode.nesting() as usize;
        let k = inode.kind_meta.fixed_size() as usize;
        nested_writer_new_body!(
            inode,
            n,
            k,
            nesting,
            [
                I8, I16, I32, I64, U8, U16, U32, U64, F16, F32, F64, Timestamp
            ]
        )
    }

    pub fn datatype(&self) -> PyroType {
        // See the matching comment on `RootWriter::datatype`.
        match self {
            NestedWriter::Map(v) => IoType::datatype(v.as_ref()),
            NestedWriter::Nested(v) => v.datatype(),
            NestedWriter::GroupNested(v) => v.datatype(),
            NestedWriter::OptionGroupNested(v) => v.datatype(),
        }
    }

    pub fn feed(&mut self, item: &'a PyroValue) -> Result<(), WireError> {
        nested_dispatch!(self, w => w.feed(item))
    }

    pub fn variable_length(&self) -> usize {
        nested_dispatch!(self, w => w.variable_length())
    }

    pub fn padding_structure(&self, padding: &mut Padding) {
        nested_dispatch!(self, w => w.padding_structure(padding))
    }

    pub fn padding_data(&self, pass: usize, padding: &mut Padding) {
        nested_dispatch!(self, w => w.padding_data(pass, padding))
    }

    pub fn write_structure(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        nested_dispatch!(self, w => w.write_structure(buffer))
    }

    pub fn write_16(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        nested_dispatch!(self, w => w.write_16(buffer))
    }

    pub fn write_8(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        nested_dispatch!(self, w => w.write_8(buffer))
    }

    pub fn write_4(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        nested_dispatch!(self, w => w.write_4(buffer))
    }

    pub fn write_2(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        nested_dispatch!(self, w => w.write_2(buffer))
    }

    pub fn write_1(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        nested_dispatch!(self, w => w.write_1(buffer))
    }

    pub fn clear(self) -> NestedWriter<'static, W> {
        match self {
            NestedWriter::Map(v) => NestedWriter::Map(v.clear().into()),
            NestedWriter::Nested(v) => NestedWriter::Nested(v.clear()),
            NestedWriter::GroupNested(v) => NestedWriter::GroupNested(v.clear()),
            NestedWriter::OptionGroupNested(v) => NestedWriter::OptionGroupNested(v.clear()),
        }
    }
}

/// The top-level [`RootWriter`]s, in the schema's structure word, which is
/// only known at runtime (from [`InodeMeta::word`]).
#[derive(Debug, Clone)]
enum Roots<'a> {
    U16(Vec<(String, RootWriter<'a, u16>)>),
    U32(Vec<(String, RootWriter<'a, u32>)>),
}

/// `$e` on whichever word's writers `$roots` holds, as `$items`.
macro_rules! roots_dispatch {
    ($roots:expr, $items:ident => $e:expr) => {
        match $roots {
            Roots::U16($items) => $e,
            Roots::U32($items) => $e,
        }
    };
}

impl<'a> Roots<'a> {
    fn clear(self) -> Roots<'static> {
        fn clear<W: StructureWord>(
            items: Vec<(String, RootWriter<'_, W>)>,
        ) -> Vec<(String, RootWriter<'static, W>)> {
            items.into_iter().map(|(k, v)| (k, v.clear())).collect()
        }
        match self {
            Roots::U16(items) => Roots::U16(clear(items)),
            Roots::U32(items) => Roots::U32(clear(items)),
        }
    }
}

pub struct ValueWriter<'a> {
    inode_meta: InodeMeta,
    inodes: Vec<Inode>,
    u16_fixed_len: usize,
    u32_fixed_len: usize,
    pyro_type: PyroType,
    roots: Roots<'a>,
}

impl ValueWriter<'static> {
    /// Mirrors the two-pass shape shared with `wire::value_readers::RowReader::new` (flatten the
    /// schema to inodes, walk them into a parent/child tree, then build
    /// bottom-up), but trusts each inode's own `nesting()`/`group_nest()`
    /// bits for depth instead of re-deriving them from the tree walk — that
    /// bookkeeping is exactly what those fields exist to replace.
    pub fn new(schema: &PyroSchema) -> Result<Self, WireError> {
        let parts = SchemaParts::from_schema(schema).map_err(|_| WireError::InvalidInodes)?;
        let inode_meta = parts.meta;
        let inodes = parts.inodes;
        let field_names = parts.field;

        if inodes.is_empty() {
            return Err(WireError::InvalidInodes);
        }

        struct NodeMeta {
            inode: Inode,
            children: Vec<usize>,
            name: Option<String>,
        }

        // ------------------------------------------------------------------
        // PASS 1: parent/child tree + field names (pre-order, non-recursive)
        // ------------------------------------------------------------------
        let mut nodes: Vec<NodeMeta> = Vec::with_capacity(inodes.len());
        // (parent_idx, remaining children to consume)
        let mut stack: Vec<(usize, usize)> = Vec::new();
        let mut root_indices: Vec<usize> = Vec::new();
        // `field_names` is 1:1 with `inodes` in this same pre-order walk,
        // except it skips `Map` children entirely (they have no entry) —
        // see `SchemaParts::push_field`/`push_group`/`push_map`.
        let mut field_cursor = 0usize;

        for &inode in inodes.iter() {
            let node_idx = nodes.len();
            let is_map_child = stack
                .last()
                .map(|&(parent_idx, _)| nodes[parent_idx].inode.kind == WireKind::Map)
                .unwrap_or(false);

            let name = if is_map_child {
                None
            } else {
                let name = field_names.get(field_cursor).map(|s| s.to_string());
                field_cursor += 1;
                name
            };

            nodes.push(NodeMeta {
                inode,
                children: Vec::new(),
                name,
            });

            if let Some((parent_idx, remaining)) = stack.last_mut() {
                nodes[*parent_idx].children.push(node_idx);
                *remaining -= 1;
            } else {
                root_indices.push(node_idx);
            }

            let num_children = match inode.kind {
                WireKind::Group | WireKind::OptionGroup => inode.group_count() as usize,
                WireKind::Map => 2,
                _ => 0,
            };
            if num_children > 0 {
                stack.push((node_idx, num_children));
            }
            while let Some(&(_, 0)) = stack.last() {
                stack.pop();
            }
        }

        // ------------------------------------------------------------------
        // PASS 2: build writers bottom-up (post-order)
        // ------------------------------------------------------------------
        // Generic over the structure word, which is only known here at runtime.
        // ------------------------------------------------------------------
        fn build<W: StructureWord>(
            nodes: &[NodeMeta],
            root_indices: &[usize],
        ) -> Result<Vec<(String, RootWriter<'static, W>)>, WireError> {
            let mut root_out: Vec<Option<RootWriter<'static, W>>> =
                (0..nodes.len()).map(|_| None).collect();
            let mut nested_out: Vec<Option<NestedWriter<'static, W>>> =
                (0..nodes.len()).map(|_| None).collect();

            for i in (0..nodes.len()).rev() {
                let inode = &nodes[i].inode;

                match inode.kind {
                    WireKind::Unknown => return Err(WireError::InvalidInodes),

                    g @ (WireKind::Group | WireKind::OptionGroup) => {
                        let nullable = matches!(g, WireKind::OptionGroup);
                        // A flat (non-repeated) group lays its fields straight
                        // into the fixed region, like a root value; anything
                        // else — `List<Group>`, or a field of a group/map that
                        // itself repeats — needs `NestedGroup`/
                        // `NestedOptionGroup` and its children fed once per
                        // instance. See the module docs on `group.rs`.
                        let flat = inode.nesting() == 0 && !inode.group_nest();

                        if flat {
                            let mut items = Vec::with_capacity(nodes[i].children.len());
                            for &c in &nodes[i].children {
                                let name = nodes[c].name.clone().unwrap_or_default();
                                let w = root_out[c].take().ok_or(WireError::InvalidInodes)?;
                                items.push((name, w));
                            }
                            root_out[i] = Some(if nullable {
                                let datatype = PyroType::OptionGroup(
                                    items.iter().map(|(_, w)| w.datatype()).collect(),
                                );
                                RootWriter::OptionGroup(OptionGroup::new(datatype, i, items))
                            } else {
                                let datatype = PyroType::Group(
                                    items.iter().map(|(_, w)| w.datatype()).collect(),
                                );
                                RootWriter::Group(Group::new(datatype, items))
                            });
                        } else {
                            let mut items = Vec::with_capacity(nodes[i].children.len());
                            for &c in &nodes[i].children {
                                let name = nodes[c].name.clone().unwrap_or_default();
                                let w = nested_out[c].take().ok_or(WireError::InvalidInodes)?;
                                items.push((name, w));
                            }
                            // `nesting - 1 + group_nest`; safe since at least one
                            // of the two is nonzero here (`flat` is false).
                            let depth = inode.nesting() as usize + inode.group_nest() as usize - 1;

                            if nullable {
                                let nog = NestedOptionGroup::new(i, depth, items);
                                if inode.group_nest() {
                                    nested_out[i] = Some(NestedWriter::OptionGroupNested(nog));
                                } else {
                                    root_out[i] = Some(RootWriter::OptionGroupNested(nog));
                                }
                            } else {
                                let ng = NestedGroup::new(i, depth, items);
                                if inode.group_nest() {
                                    nested_out[i] = Some(NestedWriter::GroupNested(ng));
                                } else {
                                    root_out[i] = Some(RootWriter::GroupNested(ng));
                                }
                            }
                        }
                    }

                    WireKind::Map => {
                        let [key, value] = nodes[i].children[..] else {
                            return Err(WireError::InvalidInodes);
                        };
                        // A map's key/value are always fed once per entry,
                        // whether the map itself sits at the root or repeats.
                        let key = nested_out[key].take().ok_or(WireError::InvalidInodes)?;
                        let value = nested_out[value].take().ok_or(WireError::InvalidInodes)?;
                        // `nesting + group_nest` (map.rs docs): its own entries
                        // are the intrinsic level, so a map already at the root
                        // can have depth 0.
                        let depth = inode.nesting() as usize + inode.group_nest() as usize;
                        let map = Map::new(i, depth, key, value);

                        if inode.group_nest() {
                            nested_out[i] = Some(NestedWriter::Map(Box::new(map)));
                        } else {
                            root_out[i] = Some(RootWriter::Map(map));
                        }
                    }

                    // Every other kind is a leaf: `RootWriter::new`/
                    // `NestedWriter::new` decide everything else (fixed-size,
                    // depth, nullability) purely from this one inode.
                    _ => {
                        if inode.group_nest() {
                            nested_out[i] = Some(NestedWriter::new(i, inode));
                        } else {
                            root_out[i] = Some(RootWriter::new(i, inode));
                        }
                    }
                }
            }

            let mut items = Vec::with_capacity(root_indices.len());
            for &idx in root_indices {
                let name = nodes[idx].name.clone().unwrap_or_default();
                let w = root_out[idx].take().ok_or(WireError::InvalidInodes)?;
                items.push((name, w));
            }
            Ok(items)
        }

        let roots = match inode_meta.word {
            Word::U16 => Roots::U16(build::<u16>(&nodes, &root_indices)?),
            Word::U32 => Roots::U32(build::<u32>(&nodes, &root_indices)?),
        };
        let pyro_type = roots_dispatch!(&roots, items => {
            PyroType::Group(items.iter().map(|(_, w)| w.datatype()).collect())
        });

        let u16_fixed_len: usize = inode_data_len::<u16>(&inodes);
        let u32_fixed_len: usize = inode_data_len::<u32>(&inodes);
        Ok(Self {
            inodes,
            pyro_type,
            roots,
            inode_meta,
            u16_fixed_len,
            u32_fixed_len,
        })
    }

    pub fn inodes(&self) -> &[Inode] {
        &self.inodes
    }

    pub fn ingest<'a>(self, item: &'a PyroValue<'a>) -> Result<ValueWriter<'a>, WireError> {
        fn ingest<'a, W: StructureWord>(
            items: &mut [(String, RootWriter<'a, W>)],
            pyro_type: &PyroType,
            item: &'a PyroValue<'a>,
        ) -> Result<(), WireError> {
            if let [(_, w)] = items {
                return w.ingest(item);
            }
            let PyroValue::Group(row) = item else {
                return Err(WireError::TypeMismatch {
                    expected: pyro_type.clone(),
                    actual: item.datatype(),
                });
            };
            for (name, w) in items {
                let value = row.get(name).ok_or(WireError::TypeMismatch {
                    expected: pyro_type.clone(),
                    actual: item.datatype(),
                })?;
                w.ingest(value)?;
            }
            Ok(())
        }

        let mut this =
            unsafe { std::mem::transmute::<ValueWriter<'static>, ValueWriter<'a>>(self) };
        roots_dispatch!(&mut this.roots, items => ingest(items, &this.pyro_type, item))?;
        Ok(this)
    }
}

impl<'a> ValueWriter<'a> {
    pub fn fixed_len(&self) -> usize {
        match self.inode_meta.word {
            Word::U16 => self.u16_fixed_len,
            Word::U32 => self.u32_fixed_len,
        }
    }

    /// Every byte [`Self::write`] appends past the fixed region, alignment
    /// padding included.
    pub fn variable_len(&self) -> usize {
        let data: usize = roots_dispatch!(&self.roots, items => {
            items.iter().map(|(_, w)| w.variable_length()).sum()
        });
        data + self.padding()
    }

    /// Alignment bytes [`Self::write`] inserts between the items' regions
    /// when it starts appending at `at`: a dry run of its passes, in the same
    /// order.
    fn padding(&self) -> usize {
        let mut padding = Padding::new(self.fixed_len());
        roots_dispatch!(&self.roots, items => {
            for (_, w) in items {
                w.padding_structure(&mut padding);
            }
            for pass in [16, 8, 4, 2, 1] {
                for (_, w) in items {
                    w.padding_data(pass, &mut padding);
                }
            }
        });
        padding.total()
    }

    /// Every item's structure (scalars inline, everything else its headers
    /// and topology), then every item's data, one alignment pass at a time.
    pub fn write<S: PyroSink>(&self, sink: &mut S) -> Result<(), WireError> {
        let len = self.fixed_len() + self.variable_len();
        match &self.roots {
            Roots::U16(_) => {
                if len >= u16::MAX as usize {
                    return Err(WireError::OffsetOverflow {
                        end: len,
                        max: u16::MAX as usize,
                    });
                }
            }
            Roots::U32(_) => {
                if len >= u32::MAX as usize {
                    return Err(WireError::OffsetOverflow {
                        end: len,
                        max: u32::MAX as usize,
                    });
                }
            }
        }
        // The sink holds the fixed region already; this sizes the rest.
        sink.reallocate_sync(self.variable_len())?;
        fn write<W: StructureWord>(
            items: &[(String, RootWriter<'_, W>)],
            buffer: &mut SplitSink<'_, W>,
        ) -> Result<(), WireError> {
            for (_, w) in items {
                w.write_structure(buffer)?;
            }
            for (_, w) in items {
                w.write_16(buffer)?;
            }
            for (_, w) in items {
                w.write_8(buffer)?;
            }
            for (_, w) in items {
                w.write_4(buffer)?;
            }
            for (_, w) in items {
                w.write_2(buffer)?;
            }
            for (_, w) in items {
                w.write_1(buffer)?;
            }
            Ok(())
        }

        match &self.roots {
            Roots::U16(items) => write(
                items,
                &mut SplitSink::new(sink, &self.inodes, self.u16_fixed_len)?,
            ),
            Roots::U32(items) => write(
                items,
                &mut SplitSink::new(sink, &self.inodes, self.u32_fixed_len)?,
            ),
        }
    }

    pub fn clear(self) -> ValueWriter<'static> {
        ValueWriter {
            inode_meta: self.inode_meta,
            inodes: self.inodes,
            pyro_type: self.pyro_type,
            roots: self.roots.clear(),
            u16_fixed_len: self.u16_fixed_len,
            u32_fixed_len: self.u32_fixed_len,
        }
    }
}
