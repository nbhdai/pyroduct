//! Reader, writer, sizing, and codec traits.

mod realize;
mod typeable;

use crate::buf::sink::LocalMemSink;
use crate::buf::{PyroBuf, PyroSink};
use crate::errors::WireError;

pub use pyro_schema::{Inode, nullable_bit_index_for, nullable_validity_start};
use pyro_schema::{InodeMeta, PrimitiveDataType, PyroType, StructureWord};
use std::str::Utf8Error;
use std::sync::atomic::AtomicUsize;

// =============================================================================
// Realize — borrowed view -> owned value
// =============================================================================

/// A borrowed view that can be copied out into its owned form.
///
/// The counterpart to [`crate::DeepRef`] (owned -> borrowed), but not its
/// type-level inverse: a wire view's shape (e.g. dense slice + validity
/// mask) need not be the shape an owned value's `DeepRef::Ref` produces.
/// See `docs/read-side-prep.md` §1.1.
///
/// Named `realize`, not `to_owned`: `ToOwned` is already implemented for
/// `&str`/`&[T]` with a different `Owned`, and `Borrow`'s contract does not
/// fit a view that owns a topology or a validity mask.
///
/// Impls for the std/bitvec view types live in `realize.rs`.
pub trait Realize<Owned> {
    fn realize(&self) -> Owned;
}

pub trait TryRealize<Owned> {
    fn try_realize(&self) -> Result<Owned, Utf8Error>;
}

// =============================================================================
// RowsRef — a column-shaped, indexable, windowable borrowed view
// =============================================================================

/// Every column-shaped view — `&'b [T]`, an option-list view, a nested
/// column, a derive-emitted `{Struct}ListRef` — is "some rows, indexable,
/// windowable". `MapRef` (`wire::readers::map`) is built from two of these.
///
/// Impls for the std/bitvec view types live in `realize.rs`.
pub trait RowsRef: Sized {
    type Row;

    fn len(&self) -> usize;
    fn get(&self, i: usize) -> Option<Self::Row>;
    /// Elements `[start, end)`. `O(1)` for a plain slice, one popcount for a
    /// validity-masked view.
    fn window(&self, start: usize, end: usize) -> Option<Self>;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn iter(&self) -> impl Iterator<Item = Self::Row> + '_ {
        (0..self.len()).map(move |i| self.get(i).unwrap())
    }
}

/// How many elements a column view holds at its outermost level — the one
/// thing every `ForeignReader::NestedRef` has at every depth, even where it
/// isn't a full [`RowsRef`]. The readers check it against the length the
/// wire encodes for the column. Impls live in `refs`, and the derive emits
/// one for every struct's column.
pub trait ColumnLen {
    fn column_len(&self) -> usize;
}

/// Const-time schema layout table for a type, populated by
/// `#[derive(PyroCodec)]` for **non-generic** structs (see
/// `docs/const-layout-plan.md`, Step 3).
///
/// The derive emits, for each such struct, a module-scope `RAW` array built
/// as a const-time literal — copying in `<Nested as Layout>::RAW` by
/// reference wherever a field is itself a derived struct — then runs
/// [`layout_in_place`] over a copy of it inside a `static` initializer to
/// produce `INODES`/`FIXED_SIZE`. Because this is a *reference* to another
/// type's associated const rather than a literal value, it composes
/// correctly regardless of macro expansion order: name resolution and const
/// evaluation both run after all macro expansion completes, and rustc
/// topologically sorts the const dependency graph.
///
/// Generic structs (any non-lifetime generic parameter) can't build
/// `[Inode; Self::RAW_LEN]` without the unstable `generic_const_exprs`
/// feature, so they don't get an impl of this trait — they stay on the
/// runtime `OnceLock`-cached path indefinitely.
/// # Const-offset nesting is not yet threaded through
///
/// The derive's promoted, default `{Struct}Prep`/`{Struct}Writer` pair
/// (`Struct::pyro_writer_prep()`) uses this trait's `INODES` to fold every
/// primitive/array/option field's offset into a compile-time-immediate
/// store. That only holds for `Struct` used as a *top-level* row type.
/// When `Struct` is embedded as a nested field inside some `Outer`, `Outer`'s
/// generated code instead constructs a `{Struct}NestedPrep`/
/// `{Struct}NestedWriter` (the original, offset-flexible `from_inode`-based
/// pair) via `Struct::pyro_writer_prep_from_inodes(&outer_inodes[...])` —
/// because `Struct`'s field offsets when nested inside `Outer` can differ
/// from `Struct`'s own standalone `Self::INODES` offsets (different
/// bucket-packing neighbors), so the promoted pair's baked-in offsets would
/// be wrong there. That fallback writer is still fully correct, just not
/// const-offset — nesting doesn't yet get the same speedup top-level rows
/// do.
///
/// Closing that gap — giving `{Struct}NestedWriter` itself const-generic
/// offset parameters threaded down from the parent's own `INODES` at the
/// parent's compile time — is deliberately deferred to a future step. It's
/// a bigger change (every nested writer would need an offset-parameterized
/// form, and composing those through multiple levels of nesting gets
/// combinatorial fast), and it's not obviously something every struct
/// should pay for unconditionally. A plausible shape for that future step:
/// an opt-in macro — something like a hypothetical `pyro_optimize! { .. }`
/// wrapper — that a caller applies to a specific struct tree when they want
/// full const-offset nesting, rather than forcing the complexity onto every
/// `#[derive(PyroCodec)]` struct whether or not it's ever nested. This note
/// is intentionally just a limitation statement, not a partial
/// implementation of that step.
pub trait Layout {
    /// The word this struct's structure (topology tiers, fixed-region
    /// pointers) is written in. [`Self::INODES`] are laid out in it.
    type Word: StructureWord;
    /// The message-wide settings that go with [`Self::INODES`]: together they
    /// are what reads a message of this type. Follows [`Self::Word`]; override
    /// it only to set a field `Word` doesn't decide.
    const META: InodeMeta = InodeMeta::new(<Self::Word as StructureWord>::WORD);
    /// Number of inodes in [`Self::RAW`] / [`Self::INODES`].
    const RAW_LEN: usize;
    /// This struct's inode tree before [`layout_in_place`] has assigned
    /// offsets — i.e. exactly what `pyro_raw_inodes()` builds at runtime.
    const RAW: &'static [Inode];
    /// This struct's inode tree after [`layout_in_place`] — exactly what
    /// `pyro_inodes()` builds at runtime, but computed once at const-eval
    /// time instead of behind a `OnceLock`.
    const INODES: &'static [Inode];
    /// This struct's total fixed-region byte count — [`layout_in_place`]'s
    /// return value, computed alongside [`Self::INODES`].
    const FIXED_SIZE: usize;

    /// Where the validity starts in the fixed buffer
    const VALIDITY_START: usize;

    /// The struct's own `///` documentation, baked in by the derive. Empty
    /// when the struct carries no doc comment.
    const DOC: &'static str;
    /// Number of entries in [`Self::FLAT_NAMES`] / [`Self::FLAT_DOCS`].
    ///
    /// Exposed as its own const — rather than left as `FLAT_NAMES.len()` —
    /// for the same reason as [`Self::RAW_LEN`]: a parent struct's const
    /// `[&'static str; N]` array literal needs the length of every nested
    /// struct's table as a const-evaluable *expression* at the point the
    /// array type is written.
    const FLAT_LEN: usize;
    /// This struct's field names flattened into the pre-order packing
    /// [`crate::PyroSchema`] uses: each field's own name, immediately
    /// followed by the names of every `Group` reachable from its type. This
    /// is what [`crate::PyroSchema::from_raw`] wants, and — unlike the
    /// top-level-only `pyro_field_names()` — it descends into nested
    /// derived structs via their own `FLAT_NAMES`.
    const FLAT_NAMES: &'static [&'static str];
    /// [`Self::FLAT_NAMES`]' parallel documentation table: entry `i` is the
    /// `///` doc comment of the field named `FLAT_NAMES[i]`, or `""`.
    const FLAT_DOCS: &'static [&'static str];

    /// This struct's type as a [`PyroType::Group`] of its top-level fields,
    /// decoded from [`Self::RAW`].
    ///
    /// A fn rather than a `const PyroType`: `PyroType` holds `Box`/`Vec`,
    /// which can't be built at const-eval time.
    const PYRO_TYPE: PyroType;

    /// [`schema_hash`](pyro_schema::schema_hash) of [`Self::META`] and
    /// [`Self::INODES`].
    const SCHEMA_HASH: u64;
}

impl<T: Layout> Typeable for T {
    fn pyro_type() -> PyroType {
        <Self as Layout>::PYRO_TYPE
    }
}

/// A trait for Rust types that can statically report their corresponding [`PyroType`].
pub trait Typeable {
    /// Returns the [`PyroType`] schema descriptor for this Rust type.
    fn pyro_type() -> PyroType;

    /// If this type corresponds to a [`PrimitiveDataType`], returns it.
    ///
    /// This is primarily used internally to determine if a `Vec<T>` should be
    /// represented as a `PrimitiveList` (optimized) or a generic `List`.
    fn primitive_data_type() -> Option<PrimitiveDataType> {
        None
    }

    /// Indicates if this type represents a nullable value (e.g., `Option<T>`).
    ///
    /// Used to determine the `nullable` flag in `PyroType::List`.
    fn is_nullable() -> bool {
        false
    }

    /// Indicates whether values of this type support repair/coercion from
    /// mismatched `PyroValue` variants (e.g., receiving an `I64` when `I32`
    /// is expected, or parsing a `Str` into a numeric).
    ///
    /// This mirrors the capabilities of [`PyroValue::repair`]: types that
    /// return `true` can be the *target* of a repair operation.
    ///
    /// Returns `false` by default. Overridden to `true` for:
    /// - All numeric primitives (integers, floats) — cross-numeric casting
    /// - `bool` — from integers (0/1) and strings ("true"/"false")
    /// - `String` / `&str` — from any scalar via `.to_string()`
    ///
    /// Notably returns `false` for:
    /// - `DateTime<Utc>` (Timestamp) — identity only, no coercion
    /// - Complex types delegate to their inner types
    fn coercible() -> bool {
        false
    }
}

pub trait Writer: Layout {
    /// The largest variable region any message of this type has needed so
    /// far, shared by every writer of the type: `encode*` preallocates this
    /// much and raises it after each write. One `static` per type (the derive
    /// emits it), since a `const` atomic would be a fresh copy at every use.
    fn max_variable() -> &'static AtomicUsize;

    fn pyro_write<S: PyroSink>(&self, buffer: &mut S) -> Result<(), WireError>;

    fn fixed_len(&self) -> usize;

    /// Length of the buffer
    fn variable_length(&self) -> usize;

    fn encode(&self) -> Result<PyroBuf, WireError> {
        let message = Self::max_variable().load(std::sync::atomic::Ordering::Relaxed);
        let mut buf = LocalMemSink::with_variable(
            <Self as Layout>::FIXED_SIZE,
            <Self as Layout>::SCHEMA_HASH,
            message,
        );
        self.pyro_write(&mut buf)?;
        Self::max_variable().fetch_max(buf.variable, std::sync::atomic::Ordering::Relaxed);
        Ok(buf.buf)
    }
}

pub trait Reader: Layout + Sized {
    type Ref<'a>: Realize<Self>;

    fn pyro_read(buffer: &[u8]) -> Result<Self::Ref<'_>, WireError>;
}

pub mod offsets {
    use std::marker::PhantomData;

    /// A compile-time index into a [`Layout`]'s inode table.
    ///
    /// The offset is carried as a *type* rather than a `const` generic parameter
    /// because associated consts may perform arithmetic on generic parameters,
    /// while const generic *arguments* may not. `O::N` still monomorphizes to a
    /// literal, so `Inodes::INODES[O::N]` folds to a constant load.
    pub trait InodeOffset {
        const N: usize;
    }

    /// The width of the thing we're measuring

    pub trait InodeWidth {
        const INODE_COUNT: usize;
    }

    // One width per type, shared by its writer and its reader. A leaf (a
    // scalar, a string, a fixed array, a bit vector) is one inode; a list or
    // an `Option` adds none of its own — the nesting or nullability rides on
    // its element's inode; a map is its own inode, then its key's and its
    // value's. A derived struct's (`1 +` its fields) comes from
    // `#[derive(Layout)]`.

    macro_rules! leaf_width {
        ($($ty:ty),* $(,)?) => {
            $(impl InodeWidth for $ty {
                const INODE_COUNT: usize = 1;
            })*
        };
    }

    leaf_width!(
        bool,
        u8,
        u16,
        u32,
        u64,
        i8,
        i16,
        i32,
        i64,
        half::f16,
        f32,
        f64,
        crate::value::Time,
        str,
        String,
        Box<str>,
        std::sync::Arc<str>,
        std::rc::Rc<str>,
        crate::BitVec,
        bytes::Bytes,
        bytes::BytesMut,
    );

    impl InodeWidth for std::borrow::Cow<'_, str> {
        const INODE_COUNT: usize = 1;
    }

    macro_rules! element_width {
        ($([$($generics:tt)*] $ty:ty),* $(,)?) => {
            $(impl<$($generics)*> InodeWidth for $ty {
                const INODE_COUNT: usize = T::INODE_COUNT;
            })*
        };
    }

    element_width!(
        [T: InodeWidth] [T],
        [T: InodeWidth] Vec<T>,
        [T: InodeWidth] Box<[T]>,
        [T: InodeWidth] std::sync::Arc<[T]>,
        [T: InodeWidth] std::rc::Rc<[T]>,
        ['a, T: InodeWidth + Clone] std::borrow::Cow<'a, [T]>,
        [T: InodeWidth + ?Sized] &T,
        [T: InodeWidth] Option<T>,
        [T: InodeWidth, const N: usize] [T; N],
    );

    macro_rules! map_width {
        ($($map:ident),* $(,)?) => {
            $(impl<K: InodeWidth, V: InodeWidth> InodeWidth for $map<K, V> {
                const INODE_COUNT: usize = 1 + K::INODE_COUNT + V::INODE_COUNT;
            })*
        };
    }

    use std::collections::BTreeMap;

    use crate::map::Map;

    map_width!(BTreeMap, Map);

    /// The root of a layout: inode zero.
    pub struct Root;

    impl InodeOffset for Root {
        const N: usize = 0;
    }

    /// `O` shifted by a literal amount — a container's own inode(s).
    pub struct Shift<O, const M: usize>(PhantomData<O>);

    impl<O: InodeOffset, const M: usize> InodeOffset for Shift<O, M> {
        const N: usize = O::N + M;
    }

    /// `O` shifted past every inode `T` occupies.
    ///
    /// `T` may be unsized: a derived struct's `Vec<Vec<u32>>` field writes
    /// through `[Vec<u32>]`'s impl, and the next field's offset is past that.
    pub struct Past<O, T: ?Sized>(PhantomData<O>, PhantomData<T>);

    impl<O: InodeOffset, T: InodeWidth + ?Sized> InodeOffset for Past<O, T> {
        const N: usize = O::N + T::INODE_COUNT;
    }
}
