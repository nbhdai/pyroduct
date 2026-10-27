//! [`MapRef`] — a borrowed `BTreeMap<K, V>` — and [`NestedMapRef`] for
//! `Vec<BTreeMap<K, V>>`, `Vec<Vec<BTreeMap<K, V>>>`, ….
//!
//! # Layout
//!
//! A map is two columns of the same length, `keys` and `values`, where
//! entry `i` is `(keys.get(i), values.get(i))`. Both are any
//! [`RowsRef`]: a `&'a [u32]`, a `StrNested<'a, W, 1>`,
//! a derive-emitted `FooSlice<'a>`, ….
//!
//! Lists of maps work like [`super::GroupNested`]: the key and value columns hold
//! *every* entry of every map in the buffer, and a [`Topology`] whose last
//! level counts entries per map says where each one starts. Narrowing to one
//! map windows both columns to its entry range; nothing is copied.
//!
//! | Rust type                    | view                             | row                              |
//! |------------------------------|----------------------------------|----------------------------------|
//! | `BTreeMap<K, V>`             | `MapRef<Ks, Vs>`                 | —                                |
//! | `Vec<BTreeMap<K, V>>`        | `NestedMapRef<'a, W, Ks, Vs, 1>`    | `MapRef<Ks, Vs>`                 |
//! | `Vec<Vec<BTreeMap<K, V>>>`   | `NestedMapRef<'a, W, Ks, Vs, 2>`    | `NestedMapRef<'a, W, Ks, Vs, 1>`    |
//! | …up to `N = 5`               |                                  |                                  |
//!
//! # Key order
//!
//! A map written from a `BTreeMap` lays its entries out in ascending key
//! order, but the wire doesn't promise it: another writer may not sort, so
//! nothing here relies on order. Lookups by key are a linear scan over the
//! key rows, `O(len)`, and "first"/"last" mean wire order.
//!
//! # Using a `MapRef`
//!
//! Like a `BTreeMap`:
//!
//! * `len()`, `is_empty()`.
//! * `get(&key)`, `get_key_value(&key)`, `contains_key(&key)` — for any `Q`
//!   the key rows compare with (`K::Row: PartialEq<Q>`): `&"a"` or
//!   `&String` against `&str` rows, `&3` against `u32` rows, `&foo`
//!   against `FooRef` rows. `get_by(|k| …)` takes any predicate.
//! * `iter()` — `(key, value)` in wire order, double-ended and exact-size;
//!   `for (k, v) in &map` works. `keys()` and `values()` are the columns'
//!   own [`RowsRef::iter`].
//! * `first_key_value()`, `last_key_value()`, `get_index(i)` — by position.
//! * `realize()` into a `BTreeMap` (when both columns realize to `Vec`s),
//!   and `==` against a `BTreeMap` or a [`crate::Map`], in both directions.

use std::collections::BTreeMap;
use std::fmt;

use crate::errors::WireError;
use crate::interface::{Realize, RowsRef};
use crate::refs::Topology;
use crate::{ColumnLen, Map};
use pyro_schema::StructureWord;

// =============================================================================
// MapRef — one map
// =============================================================================

/// A borrowed `BTreeMap`: entry `i` is `(keys.get(i), values.get(i))` — see
/// the [module doc](self).
///
/// `keys` and `values` must be the same length; [`MapRef::new`] checks it.
#[derive(Clone, Copy)]
pub struct MapRef<K, V> {
    pub keys: K,
    pub values: V,
}

impl<K: ColumnLen, V: ColumnLen> MapRef<K, V> {
    /// [`WireError::LengthMismatch`] when the columns' lengths differ:
    /// the keys' length is the one expected, and the values (column `1`)
    /// are the one that disagrees.
    pub fn new(len: usize, keys: K, values: V) -> Result<Self, WireError> {
        if len != keys.column_len() {
            return Err(WireError::LengthMismatch {
                expected: len,
                have: keys.column_len(),
                index: 0,
            });
        }

        if len != values.column_len() {
            return Err(WireError::LengthMismatch {
                expected: len,
                have: values.column_len(),
                index: 1,
            });
        }
        Ok(MapRef { keys, values })
    }
}

impl<K: RowsRef, V: RowsRef> MapRef<K, V> {
    /// Number of entries.
    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Index of the first entry whose key equals `key`.
    fn position<Q: ?Sized>(&self, key: &Q) -> Option<usize>
    where
        K::Row: PartialEq<Q>,
    {
        RowsRef::iter(&self.keys).position(|k| k == *key)
    }

    /// The value for `key`, from the first entry holding it. `O(len)`: see
    /// the [module doc](self#key-order).
    pub fn get<Q: ?Sized>(&self, key: &Q) -> Option<V::Row>
    where
        K::Row: PartialEq<Q>,
    {
        self.values.get(self.position(key)?)
    }

    /// The first entry holding `key`, as `(key, value)`.
    pub fn get_key_value<Q: ?Sized>(&self, key: &Q) -> Option<(K::Row, V::Row)>
    where
        K::Row: PartialEq<Q>,
    {
        self.get_index(self.position(key)?)
    }

    pub fn contains_key<Q: ?Sized>(&self, key: &Q) -> bool
    where
        K::Row: PartialEq<Q>,
    {
        self.position(key).is_some()
    }

    /// The value of the first entry whose key satisfies `pred`.
    pub fn get_by(&self, mut pred: impl FnMut(&K::Row) -> bool) -> Option<V::Row> {
        self.values
            .get(RowsRef::iter(&self.keys).position(|k| pred(&k))?)
    }

    /// Entry `i` in wire order. `None` when out of bounds.
    pub fn get_index(&self, i: usize) -> Option<(K::Row, V::Row)> {
        Some((self.keys.get(i)?, self.values.get(i)?))
    }

    /// The first entry in wire order — the smallest key when the map was
    /// written from a `BTreeMap`, but see the [module doc](self#key-order).
    pub fn first_key_value(&self) -> Option<(K::Row, V::Row)> {
        self.get_index(0)
    }

    /// The last entry in wire order.
    pub fn last_key_value(&self) -> Option<(K::Row, V::Row)> {
        self.get_index(self.len().checked_sub(1)?)
    }

    /// Entries `[start, end)` in wire order, as a map of their own.
    pub fn window(&self, start: usize, end: usize) -> Option<Self> {
        Some(MapRef {
            keys: self.keys.window(start, end)?,
            values: self.values.window(start, end)?,
        })
    }

    /// Every entry as `(key, value)`, in wire order.
    pub fn iter(&self) -> MapIter<K, V>
    where
        K: Clone,
        V: Clone,
    {
        self.clone().into_iter()
    }

    /// Every key, in wire order.
    pub fn keys(&self) -> impl Iterator<Item = K::Row> + '_ {
        RowsRef::iter(&self.keys)
    }

    /// Every value, in wire order.
    pub fn values(&self) -> impl Iterator<Item = V::Row> + '_ {
        RowsRef::iter(&self.values)
    }
}

/// A repeated key keeps its last value, as `BTreeMap::from_iter` does.
impl<K, V, KO, VO> Realize<BTreeMap<KO, VO>> for MapRef<K, V>
where
    K: Realize<Vec<KO>>,
    V: Realize<Vec<VO>>,
    KO: Ord,
{
    fn realize(&self) -> BTreeMap<KO, VO> {
        self.keys
            .realize()
            .into_iter()
            .zip(self.values.realize())
            .collect()
    }
}

impl<K: RowsRef + Clone, V: RowsRef + Clone> MapRef<K, V> {
    /// Equal when both hold the same number of entries and every owned
    /// entry is found here with an equal value. Entries in the same order
    /// are matched position by position; out-of-order ones fall back to a
    /// search, so an unsorted map still compares equal. Keys here are
    /// assumed unique, as an owned map's are.
    fn eq_entries<'o, KO: 'o, VO: 'o>(
        &self,
        len: usize,
        entries: impl Iterator<Item = (&'o KO, &'o VO)>,
    ) -> bool
    where
        K::Row: PartialEq<KO>,
        V::Row: PartialEq<VO>,
    {
        self.len() == len
            && entries.enumerate().all(|(i, (k, v))| {
                let at = match self.keys.get(i) {
                    Some(key) if key == *k => Some(i),
                    _ => self.position(k),
                };
                at.and_then(|at| self.values.get(at))
                    .is_some_and(|value| value == *v)
            })
    }
}

/// `==` against an owned sorted map, via `MapRef::eq_entries`.
macro_rules! map_ref_eq {
    ($owned:ident) => {
        impl<K, V, KO, VO> PartialEq<$owned<KO, VO>> for MapRef<K, V>
        where
            K: RowsRef + Clone,
            V: RowsRef + Clone,
            K::Row: PartialEq<KO>,
            V::Row: PartialEq<VO>,
        {
            fn eq(&self, other: &$owned<KO, VO>) -> bool {
                self.eq_entries(other.len(), other.iter())
            }
        }

        impl<K, V, KO, VO> PartialEq<MapRef<K, V>> for $owned<KO, VO>
        where
            K: RowsRef + Clone,
            V: RowsRef + Clone,
            K::Row: PartialEq<KO>,
            V::Row: PartialEq<VO>,
        {
            fn eq(&self, other: &MapRef<K, V>) -> bool {
                *other == *self
            }
        }
    };
}

map_ref_eq!(BTreeMap);
map_ref_eq!(Map);

impl<K, V> fmt::Debug for MapRef<K, V>
where
    K: RowsRef + Clone,
    V: RowsRef + Clone,
    K::Row: fmt::Debug,
    V::Row: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

impl<K: RowsRef + Clone, V: RowsRef + Clone> IntoIterator for MapRef<K, V> {
    type Item = (K::Row, V::Row);
    type IntoIter = MapIter<K, V>;

    fn into_iter(self) -> Self::IntoIter {
        MapIter {
            front: 0,
            back: self.len(),
            map: self,
        }
    }
}

impl<'m, K: RowsRef + Clone, V: RowsRef + Clone> IntoIterator for &'m MapRef<K, V> {
    type Item = (K::Row, V::Row);
    type IntoIter = MapIter<K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// A [`MapRef`]'s entries as `(key, value)`, in wire order (or reverse).
#[derive(Clone)]
pub struct MapIter<K, V> {
    map: MapRef<K, V>,
    front: usize,
    back: usize,
}

impl<K: RowsRef + Clone, V: RowsRef + Clone> Iterator for MapIter<K, V> {
    type Item = (K::Row, V::Row);

    fn next(&mut self) -> Option<Self::Item> {
        if self.front >= self.back {
            return None;
        }
        let i = self.front;
        self.front += 1;
        self.map.get_index(i)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.back - self.front;
        (n, Some(n))
    }
}

impl<K: RowsRef + Clone, V: RowsRef + Clone> DoubleEndedIterator for MapIter<K, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.front >= self.back {
            return None;
        }
        self.back -= 1;
        self.map.get_index(self.back)
    }
}

impl<K: RowsRef + Clone, V: RowsRef + Clone> ExactSizeIterator for MapIter<K, V> {}

// =============================================================================
// NestedMapRef — `N` `Vec`s over a map column
// =============================================================================

/// `N` extra `Vec`s over a column of maps — see the [module doc](self).
///
/// * `keys`, `values` — *every* entry of every map in the buffer. Only a
///   `NestedMapRef<W, _, _, 1>` narrowing to one map windows them, to that
///   map's entry range.
/// * `topology` — `N` levels; the last one holds per-map entry boundaries,
///   each level above it groups the level below, as for [`super::Nest`].
#[derive(Debug, Clone, Copy)]
pub struct NestedMapRef<'a, W: StructureWord, K, V, const N: usize> {
    pub keys: K,
    pub values: V,
    pub topology: Topology<'a, W, N>,
}

impl<'a, W: StructureWord, K: ColumnLen, V: ColumnLen, const N: usize>
    NestedMapRef<'a, W, K, V, N>
{
    /// [`WireError::LengthMismatch`] when the columns' lengths differ:
    /// the keys' length is the one expected, and the values (column `1`)
    /// are the one that disagrees.
    pub fn new(topology: Topology<'a, W, N>, keys: K, values: V) -> Result<Self, WireError> {
        let len = topology.total();
        if len != keys.column_len() {
            return Err(WireError::LengthMismatch {
                expected: len,
                have: keys.column_len(),
                index: 0,
            });
        }

        if len != values.column_len() {
            return Err(WireError::LengthMismatch {
                expected: len,
                have: values.column_len(),
                index: 1,
            });
        }
        Ok(NestedMapRef {
            keys,
            values,
            topology,
        })
    }
}

impl<'a, W: StructureWord, K: RowsRef + Clone, V: RowsRef + Clone> NestedMapRef<'a, W, K, V, 1> {
    /// Map `i`: both columns windowed to its entries. `None` when out of
    /// bounds.
    pub fn get(&self, i: usize) -> Option<MapRef<K, V>> {
        let (start, end) = self.topology.leaf_range(i)?;
        MapRef {
            keys: self.keys.clone(),
            values: self.values.clone(),
        }
        .window(start, end)
    }
}

impl<'a, W: StructureWord, K, V, O> Realize<Vec<O>> for NestedMapRef<'a, W, K, V, 1>
where
    K: RowsRef + Clone,
    V: RowsRef + Clone,
    MapRef<K, V>: Realize<O>,
{
    fn realize(&self) -> Vec<O> {
        self.iter().map(|map| map.realize()).collect()
    }
}

macro_rules! nested_map_level {
    ($n:literal => $m:literal) => {
        impl<'a, W: StructureWord, K: RowsRef + Clone, V: RowsRef + Clone>
            NestedMapRef<'a, W, K, V, $n>
        {
            /// List `i`, one level down, `O(1)`. `None` when out of bounds.
            pub fn get(&self, i: usize) -> Option<NestedMapRef<'a, W, K, V, $m>> {
                Some(NestedMapRef {
                    keys: self.keys.clone(),
                    values: self.values.clone(),
                    topology: self.topology.descend(i)?,
                })
            }

            /// Alias for [`Self::get`].
            pub fn descend(&self, i: usize) -> Option<NestedMapRef<'a, W, K, V, $m>> {
                self.get(i)
            }
        }

        impl<'a, W: StructureWord, K, V, O> Realize<Vec<O>> for NestedMapRef<'a, W, K, V, $n>
        where
            K: RowsRef + Clone,
            V: RowsRef + Clone,
            NestedMapRef<'a, W, K, V, $m>: Realize<O>,
        {
            fn realize(&self) -> Vec<O> {
                self.iter().map(|child| child.realize()).collect()
            }
        }
    };
}

nested_map_level!(2 => 1);
nested_map_level!(3 => 2);
nested_map_level!(4 => 3);
nested_map_level!(5 => 4);

/// Iterator over a [`NestedMapRef`]'s outermost level; yields what `get(i)`
/// would, front to back (or back to front).
pub struct NestedMapIter<'a, W: StructureWord, K, V, const N: usize> {
    nest: NestedMapRef<'a, W, K, V, N>,
    front: usize,
    back: usize,
}

macro_rules! nested_map_common {
    ($($n:literal => $row:ty),* $(,)?) => {
        $(
            impl<'a, W: StructureWord, K: RowsRef + Clone, V: RowsRef + Clone> NestedMapRef<'a, W, K, V, $n> {
                /// Number of elements at this view's outermost level.
                pub fn len(&self) -> usize {
                    self.topology.len()
                }

                pub fn is_empty(&self) -> bool {
                    self.len() == 0
                }

                /// Elements `[start, end)`, same depth, `O(1)`: only the
                /// topology narrows. `None` when out of bounds.
                pub fn window(&self, start: usize, end: usize) -> Option<Self> {
                    Some(NestedMapRef {
                        keys: self.keys.clone(),
                        values: self.values.clone(),
                        topology: self.topology.window(start, end)?,
                    })
                }

                pub fn iter(&self) -> NestedMapIter<'a, W, K, V, $n> {
                    NestedMapIter {
                        nest: self.clone(),
                        front: 0,
                        back: self.len(),
                    }
                }
            }

            impl<'a, W: StructureWord, K: RowsRef + Clone, V: RowsRef + Clone> RowsRef for NestedMapRef<'a, W, K, V, $n> {
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

            impl<'a, W: StructureWord, K: RowsRef + Clone, V: RowsRef + Clone> Iterator for NestedMapIter<'a, W, K, V, $n> {
                type Item = $row;

                fn next(&mut self) -> Option<Self::Item> {
                    if self.front >= self.back {
                        return None;
                    }
                    let i = self.front;
                    self.front += 1;
                    self.nest.get(i)
                }

                fn size_hint(&self) -> (usize, Option<usize>) {
                    let n = self.back - self.front;
                    (n, Some(n))
                }
            }

            impl<'a, W: StructureWord, K: RowsRef + Clone, V: RowsRef + Clone> DoubleEndedIterator
                for NestedMapIter<'a, W, K, V, $n>
            {
                fn next_back(&mut self) -> Option<Self::Item> {
                    if self.front >= self.back {
                        return None;
                    }
                    self.back -= 1;
                    self.nest.get(self.back)
                }
            }

            impl<'a, W: StructureWord, K: RowsRef + Clone, V: RowsRef + Clone> ExactSizeIterator
                for NestedMapIter<'a, W, K, V, $n>
            {
            }

            impl<'a, W: StructureWord, K: RowsRef + Clone, V: RowsRef + Clone> IntoIterator for NestedMapRef<'a, W, K, V, $n> {
                type Item = $row;
                type IntoIter = NestedMapIter<'a, W, K, V, $n>;

                fn into_iter(self) -> Self::IntoIter {
                    self.iter()
                }
            }

            impl<'a, 'n, W: StructureWord, K: RowsRef + Clone, V: RowsRef + Clone> IntoIterator
                for &'n NestedMapRef<'a, W, K, V, $n>
            {
                type Item = $row;
                type IntoIter = NestedMapIter<'a, W, K, V, $n>;

                fn into_iter(self) -> Self::IntoIter {
                    self.iter()
                }
            }

            impl<'a, W: StructureWord, K, V, Rhs> PartialEq<[Rhs]> for NestedMapRef<'a, W, K, V, $n>
            where
                K: RowsRef + Clone,
                V: RowsRef + Clone,
                $row: PartialEq<Rhs>,
            {
                fn eq(&self, other: &[Rhs]) -> bool {
                    self.len() == other.len() && self.iter().zip(other).all(|(a, b)| a == *b)
                }
            }

            impl<'a, W: StructureWord, K, V, Rhs> PartialEq<Vec<Rhs>> for NestedMapRef<'a, W, K, V, $n>
            where
                K: RowsRef + Clone,
                V: RowsRef + Clone,
                $row: PartialEq<Rhs>,
            {
                fn eq(&self, other: &Vec<Rhs>) -> bool {
                    *self == *other.as_slice()
                }
            }

            impl<'a, W: StructureWord, K, V, Rhs> PartialEq<NestedMapRef<'a, W, K, V, $n>> for Vec<Rhs>
            where
                K: RowsRef + Clone,
                V: RowsRef + Clone,
                $row: PartialEq<Rhs>,
            {
                fn eq(&self, other: &NestedMapRef<'a, W, K, V, $n>) -> bool {
                    *other == *self.as_slice()
                }
            }
        )*
    };
}

nested_map_common!(
    1 => MapRef<K, V>,
    2 => NestedMapRef<'a, W, K, V, 1>,
    3 => NestedMapRef<'a, W, K, V, 2>,
    4 => NestedMapRef<'a, W, K, V, 3>,
    5 => NestedMapRef<'a, W, K, V, 4>,
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::refs::StrNested;
    use pyro_schema::StructureWord;

    const R: u32 = <u32 as StructureWord>::RESET;

    // Three maps, keys sorted within each:
    //   {"a": 1, "bc": 2}, {}, {"d": 3, "e": 4, "f": 5}
    const KEY_BYTES: &[u8] = b"abcdef";
    const KEY_TOPO: [u32; 6] = [R, 1, 3, 4, 5, 6];
    const VALUES: [u32; 5] = [1, 2, 3, 4, 5];

    fn keys() -> StrNested<'static, u32, 1> {
        // SAFETY: `KEY_BYTES` is ASCII and `KEY_TOPO` ends at its length.
        unsafe {
            StrNested::from_validated(
                Topology {
                    topology: [&KEY_TOPO],
                },
                KEY_BYTES,
            )
        }
    }

    fn values() -> &'static [u32] {
        &VALUES
    }

    fn owned() -> Vec<BTreeMap<String, u32>> {
        let map = |entries: &[(&str, u32)]| {
            entries
                .iter()
                .map(|&(k, v)| (k.to_owned(), v))
                .collect::<BTreeMap<_, _>>()
        };
        vec![
            map(&[("a", 1), ("bc", 2)]),
            map(&[]),
            map(&[("d", 3), ("e", 4), ("f", 5)]),
        ]
    }

    #[test]
    fn map_ref_is_a_btree_map() {
        let map = MapRef::new(5, keys(), values()).unwrap();
        assert_eq!(map.len(), 5);
        assert_eq!(map.get(&"bc"), Some(2));
        assert_eq!(map.get(&String::from("f")), Some(5));
        assert_eq!(map.get(&"b"), None);
        assert!(map.contains_key(&"d"));
        assert_eq!(map.get_key_value(&"e"), Some(("e", 4)));
        assert_eq!(map.get_by(|k| *k == "a"), Some(1));
        assert_eq!(map.first_key_value().unwrap().1, 1);
        assert_eq!(map.last_key_value().unwrap().1, 5);
        assert_eq!(map.values().collect::<Vec<_>>(), [1, 2, 3, 4, 5]);
        assert_eq!(map.iter().rev().next(), Some(("f", 5)));
        assert_eq!(map.keys().collect::<Vec<_>>(), ["a", "bc", "d", "e", "f"]);
        let entries: Vec<(&str, u32)> = (&map).into_iter().collect();
        assert_eq!(entries, [("a", 1), ("bc", 2), ("d", 3), ("e", 4), ("f", 5)]);
        assert_eq!(format!("{:?}", map.window(0, 1).unwrap()), r#"{"a": 1}"#);

        let owned: BTreeMap<String, u32> = owned().into_iter().flatten().collect();
        assert_eq!(map, owned);
        assert_eq!(owned, map);
        assert_eq!(map.realize(), owned);
        let vals: &[u32] = &VALUES[1..];
        assert!(matches!(
            MapRef::new(5, keys(), vals),
            Err(WireError::LengthMismatch {
                expected: 5,
                have: 4,
                index: 1
            })
        ));
    }

    #[test]
    fn unordered_keys_still_look_up_and_compare() {
        let values = [3u32, 1, 2];
        let map: MapRef<&[u16], &[u32]> = MapRef {
            keys: &[30u16, 10, 20],
            values: &values,
        };
        assert_eq!(map.get(&10), Some(1));
        assert_eq!(map.get(&20), Some(2));
        assert_eq!(map.first_key_value(), Some((30, 3)));
        let owned = BTreeMap::from([(10u16, 1u32), (20, 2), (30, 3)]);
        assert_eq!(map, owned);
        assert_eq!(map.realize(), owned);
        assert_ne!(map, BTreeMap::from([(10u16, 1u32), (20, 2), (30, 4)]));
        assert_ne!(map, BTreeMap::from([(10u16, 1u32), (20, 2)]));
    }

    #[test]
    fn list_of_maps() {
        let rows = [R, 2, 2, 5];
        let nest: NestedMapRef<'_, u32, _, _, 1> = NestedMapRef {
            keys: keys(),
            values: values(),
            topology: Topology { topology: [&rows] },
        };
        let owned = owned();
        assert_eq!(nest.len(), 3);
        assert_eq!(nest, owned);
        assert_eq!(owned, nest);
        assert!(nest.get(1).unwrap().is_empty());
        assert_eq!(nest.get(2).unwrap().get(&"e"), Some(4));
        assert_eq!(nest.get(2).unwrap().get(&"a"), None);
        assert!(nest.get(3).is_none());
        assert_eq!(nest.iter().rev().next().unwrap(), owned[2]);
        assert_eq!(nest.window(1, 3).unwrap(), owned[1..].to_vec());
        assert_eq!(RowsRef::get(&nest, 0).unwrap(), owned[0]);
    }

    #[test]
    fn deeper_lists_of_maps() {
        // [[m0, m1], [m2]]
        let l0 = [R, 2, 3];
        let l1 = [R, 2, 2, 5];
        let keys: &[u8] = &[1u8, 2, 3, 4, 5];
        let nest: NestedMapRef<'_, u32, _, _, 2> = NestedMapRef {
            keys,
            values: values(),
            topology: Topology {
                topology: [&l0, &l1],
            },
        };
        let map = |entries: &[(u8, u32)]| entries.iter().copied().collect::<BTreeMap<_, _>>();
        let expected = vec![
            vec![map(&[(1, 1), (2, 2)]), map(&[])],
            vec![map(&[(3, 3), (4, 4), (5, 5)])],
        ];
        assert_eq!(nest, expected);
        assert_eq!(nest.realize(), expected);
        assert_eq!(nest.get(1).unwrap().get(0).unwrap().get(&4), Some(4));
        assert_eq!(nest.descend(0).unwrap().len(), 2);
    }
}
