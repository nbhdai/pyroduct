//! [`GroupNested`] — `Vec<Vec<Group>>`, `Vec<Vec<Vec<Group>>>`, … over a group
//! column.
//!
//! # How group views nest
//!
//! A derived struct `Foo` gets two views:
//!
//! * `FooRef<'a>` — one `Foo`. Each field is that field's own view.
//! * `FooSlice<'a>` — a column of `Foo`s, i.e. a borrowed `Vec<Foo>`. Each
//!   field is the *column* of that field, one level deeper than in
//!   `FooRef`. `FooSlice` implements [`RowsRef`](crate::interface::RowsRef)
//!   with `Row = FooRef<'a>` (`get(i)` is `get(i)` on every field column)
//!   and realizes to a `Vec<Foo>`.
//!
//! ```text
//! struct Foo { a: f32, b: String, c: Vec<Vec<u32>> }
//!
//! struct FooRef<'a>   { a: f32,          b: &'a str,              c: Nested<'a, W, u32, 1> }
//! struct FooSlice<'a> { a: &'a [f32],    b: StrNested<'a, W, 1>,  c: Nested<'a, W, u32, 2> }
//! ```
//!
//! # Everything stays flat
//!
//! `FooSlice`'s field columns hold *every* `Foo` in the buffer, however
//! deeply the `Foo`s are nested. More `Vec`s above a group never touch the
//! group's columns: they add topology levels whose last level counts `Foo`
//! rows, and narrowing to one list is `FooSlice::window(start, end)`, which
//! windows each field column. `GroupNested` is exactly those extra levels:
//!
//! | Rust type             | view                         | row                          |
//! |-----------------------|------------------------------|------------------------------|
//! | `Vec<Foo>`            | `FooSlice<'a>`               | `FooRef<'a>`                 |
//! | `Vec<Vec<Foo>>`       | `GroupNested<'a, W, FooSlice<'a>, 1>`  | `FooSlice<'a>`               |
//! | `Vec<Vec<Vec<Foo>>>`  | `GroupNested<'a, W, FooSlice<'a>, 2>`  | `GroupNested<'a, W, FooSlice<'a>, 1>`  |
//! | …up to `N = 5`        |                              |                              |
//!
//! `N` counts the `Vec`s above the group column, matching
//! [`super::Nested`]. Nullable groups use [`super::OptionList`]
//! (`Vec<Option<Foo>>`) and [`super::OptionNest`] (deeper) in the same
//! position.
//!
//! # Containing structs
//!
//! A struct holding a `Vec<Foo>` field follows the same rule one level up.
//! For `struct Bar { f: Vec<Foo> }`:
//!
//! ```text
//! struct BarRef<'a>   { f: FooSlice<'a> }               // one Bar: its Vec<Foo>
//! struct BarSlice<'a> { f: GroupNested<'a, W, FooSlice<'a>, 1> }  // Vec<Bar>: a Vec<Vec<Foo>> column
//! ```
//!
//! `BarSlice`'s row `i` is `BarRef { f: <list i of self.f> }`. A `struct Var
//! { b: Vec<Bar> }` then has `VarRef { b: BarSlice<'a> }` and a `VarSlice`
//! whose `b` is `BarSlice` with one more level on each of its fields, and so
//! on up. No level ever copies or re-bases the ones below it.
//!
//! # Using a `GroupNested`
//!
//! Like a `Vec` of `Vec`s: `len`, `get(i)` (the next level down, or at
//! `N = 1` the group column windowed to list `i`), `window(start, end)`
//! (same depth), and `iter()` over the outermost level — double-ended and
//! exact-size. Every step is `O(1)`; nothing is copied.
//!
//! `GroupNested`s compare with `==` against other `GroupNested`s and against
//! `Vec`s (both directions) whenever the group column does:
//! `GroupNested<'a, W, FooSlice, 1> == Vec<Vec<Foo>>` holds when
//! `FooSlice: PartialEq<Vec<Foo>>`.

use crate::{
    ColumnLen,
    errors::WireError,
    interface::{Realize, RowsRef},
    refs::Topology,
};
use pyro_schema::StructureWord;

/// `N` extra `Vec`s over a group column `T` — see the [module doc](self).
///
/// * `rows` — the *whole* group column (every row in the buffer), e.g. a
///   `FooSlice<'a>`. Never re-based: only a `GroupNested<W, _, 1>` narrowing to one
///   list windows it, to that list's row range.
/// * `topology` — `N` levels; the last one holds row boundaries into
///   `rows`, each level above it groups the level below, exactly as for
///   [`super::Nested`]. `N` is at least 1: a bare `Vec<Foo>` is the group
///   column itself.
#[derive(Debug, Clone, Copy)]
pub struct GroupNested<'a, W: StructureWord, T, const N: usize> {
    pub(super) topology: Topology<'a, W, N>,
    rows: T,
}

impl<'a, W: StructureWord, T: ColumnLen, const N: usize> GroupNested<'a, W, T, N> {
    /// [`WireError::InvalidTopologyLen`] when the last tier's total isn't
    /// the column's length.
    pub fn new(topology: Topology<'a, W, N>, rows: T) -> Result<Self, WireError> {
        if topology.total() != rows.column_len() {
            return Err(WireError::InvalidTopologyLen {
                topo: topology.total(),
                data: rows.column_len(),
            });
        }
        Ok(Self { topology, rows })
    }
}

// =============================================================================
// Levels 1..=6 — everything that doesn't depend on the row type
// =============================================================================

macro_rules! list_common {
    ($($n:literal => $row:ty),* $(,)?) => {
        $(
            impl<'a, W: StructureWord, T: RowsRef + Clone> GroupNested<'a, W, T, $n> {
                /// Number of lists at this view's outermost level.
                pub fn len(&self) -> usize {
                    self.topology.len()
                }

                pub fn is_empty(&self) -> bool {
                    self.len() == 0
                }

                /// GroupNesteds `[start, end)`, same depth, `O(1)`: only the
                /// topology narrows; `rows` is shared. `None` when out of
                /// bounds.
                pub fn window(&self, start: usize, end: usize) -> Option<Self> {
                    Some(GroupNested {
                        topology: self.topology.window(start, end)?,
                        rows: self.rows.clone(),
                    })
                }

                /// Iterates this view's outermost level, yielding what
                /// `get(i)` would.
                pub fn iter(&self) -> GroupNestedIter<'a, W, T, $n> {
                    GroupNestedIter {
                        list: self.clone(),
                        front: 0,
                        back: self.len(),
                    }
                }
            }

            impl<'a, W: StructureWord, T: RowsRef + Clone> Iterator for GroupNestedIter<'a, W, T, $n> {
                type Item = $row;

                #[cfg_attr(
                    feature = "trace-logging",
                    tracing::instrument(level = "trace", skip_all)
                )]
                fn next(&mut self) -> Option<Self::Item> {
                    if self.front >= self.back {
                        return None;
                    }
                    let i = self.front;
                    self.front += 1;
                    self.list.get(i)
                }

                fn size_hint(&self) -> (usize, Option<usize>) {
                    let n = self.back - self.front;
                    (n, Some(n))
                }
            }

            impl<'a, W: StructureWord, T: RowsRef + Clone> DoubleEndedIterator for GroupNestedIter<'a, W, T, $n> {
                fn next_back(&mut self) -> Option<Self::Item> {
                    if self.front >= self.back {
                        return None;
                    }
                    self.back -= 1;
                    self.list.get(self.back)
                }
            }

            impl<'a, W: StructureWord, T: RowsRef + Clone> ExactSizeIterator for GroupNestedIter<'a, W, T, $n> {}

            impl<'a, W: StructureWord, T: RowsRef + Clone> RowsRef for GroupNested<'a, W, T, $n> {
                type Row = $row;

                fn len(&self) -> usize {
                    Self::len(self)
                }

                fn get(&self, i: usize) -> Option<$row> {
                    Self::get(self, i)
                }

                fn window(&self, start: usize, end: usize) -> Option<Self> {
                    Self::window(self, start, end)
                }
            }

            impl<'a, W: StructureWord, T: RowsRef + Clone> IntoIterator for GroupNested<'a, W, T, $n> {
                type Item = $row;
                type IntoIter = GroupNestedIter<'a, W, T, $n>;

                fn into_iter(self) -> Self::IntoIter {
                    self.iter()
                }
            }

            impl<'a, 'n, W: StructureWord, T: RowsRef + Clone> IntoIterator for &'n GroupNested<'a, W, T, $n> {
                type Item = $row;
                type IntoIter = GroupNestedIter<'a, W, T, $n>;

                fn into_iter(self) -> Self::IntoIter {
                    self.iter()
                }
            }

            impl<'a, 'b, W: StructureWord, T, U> PartialEq<GroupNested<'b, W, U, $n>> for GroupNested<'a, W, T, $n>
            where
                T: RowsRef + Clone + PartialEq<U>,
                U: RowsRef + Clone,
            {
                fn eq(&self, other: &GroupNested<'b, W, U, $n>) -> bool {
                    self.len() == other.len()
                        && self.iter().zip(other.iter()).all(|(a, b)| a == b)
                }
            }

            impl<'a, W: StructureWord, T: RowsRef + Clone + Eq> Eq for GroupNested<'a, W, T, $n> {}

            impl<'a, W: StructureWord, T, V> PartialEq<[V]> for GroupNested<'a, W, T, $n>
            where
                T: RowsRef + Clone,
                $row: PartialEq<V>,
            {
                fn eq(&self, other: &[V]) -> bool {
                    self.len() == other.len() && self.iter().zip(other).all(|(a, b)| a == *b)
                }
            }

            impl<'a, W: StructureWord, T, V> PartialEq<Vec<V>> for GroupNested<'a, W, T, $n>
            where
                T: RowsRef + Clone,
                $row: PartialEq<V>,
            {
                fn eq(&self, other: &Vec<V>) -> bool {
                    *self == *other.as_slice()
                }
            }

            impl<'a, W: StructureWord, T, V> PartialEq<GroupNested<'a, W, T, $n>> for Vec<V>
            where
                T: RowsRef + Clone,
                $row: PartialEq<V>,
            {
                fn eq(&self, other: &GroupNested<'a, W, T, $n>) -> bool {
                    *other == *self.as_slice()
                }
            }
        )*
    };
}

// =============================================================================
// Row types: level 1 yields the windowed group column, levels 2..=6 the
// next `GroupNested` down
// =============================================================================

impl<'a, W: StructureWord, T: RowsRef + Clone> GroupNested<'a, W, T, 1> {
    /// GroupNested `i`: the group column windowed to that list's rows (a
    /// `FooSlice` for `GroupNested<'a, W, FooSlice<'a>, 1>`). `None` when out of
    /// bounds.
    pub fn get(&self, i: usize) -> Option<T> {
        let (start, end) = self.topology.leaf_range(i)?;
        self.rows.window(start, end)
    }
}

/// `Vec<Vec<Foo>>`: every list realizes through the group column's own
/// `Realize<Vec<Foo>>` (a `FooSlice`'s).
impl<'a, W: StructureWord, T, O> Realize<Vec<O>> for GroupNested<'a, W, T, 1>
where
    T: RowsRef + Clone + Realize<O>,
{
    fn realize(&self) -> Vec<O> {
        self.iter().map(|rows| rows.realize()).collect()
    }
}

macro_rules! list_level {
    ($n:literal => $m:literal) => {
        impl<'a, W: StructureWord, T: RowsRef + Clone> GroupNested<'a, W, T, $n> {
            /// GroupNested `i`, one level down, `O(1)`. `None` when out of bounds.
            pub fn get(&self, i: usize) -> Option<GroupNested<'a, W, T, $m>> {
                Some(GroupNested {
                    topology: self.topology.descend(i)?,
                    rows: self.rows.clone(),
                })
            }

            /// Alias for [`Self::get`].
            pub fn descend(&self, i: usize) -> Option<GroupNested<'a, W, T, $m>> {
                self.get(i)
            }
        }

        impl<'a, W: StructureWord, T, O> Realize<Vec<O>> for GroupNested<'a, W, T, $n>
        where
            T: RowsRef + Clone,
            GroupNested<'a, W, T, $m>: Realize<O>,
        {
            fn realize(&self) -> Vec<O> {
                self.iter().map(|list| list.realize()).collect()
            }
        }
    };
}

list_level!(2 => 1);
list_level!(3 => 2);
list_level!(4 => 3);
list_level!(5 => 4);

list_common!(
    1 => T,
    2 => GroupNested<'a, W, T, 1>,
    3 => GroupNested<'a, W, T, 2>,
    4 => GroupNested<'a, W, T, 3>,
    5 => GroupNested<'a, W, T, 4>,
);

/// Iterator over a [`GroupNested`]'s outermost level; yields what `get(i)`
/// would, front to back (or back to front).
pub struct GroupNestedIter<'a, W: StructureWord, T, const N: usize> {
    list: GroupNested<'a, W, T, N>,
    front: usize,
    back: usize,
}
