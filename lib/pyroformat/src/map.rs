//! [`Map`] — a sorted, `Vec`-backed map with `BTreeMap`'s semantics.
//!
//! Keys are kept sorted and unique, exactly as in a `BTreeMap`, so a `Map`
//! writes the same wire bytes a `BTreeMap` with the same entries does, and
//! the two are interchangeable as field types. What changes is the storage:
//! two parallel `Vec`s, `keys` and `values`, instead of a tree of 11-slot
//! nodes. That makes small maps (the common case in records) a pair of
//! exact-size allocations instead of a whole node, and makes `keys()` and
//! `values()` — the two passes the writer makes over every map — plain slice
//! walks.
//!
//! | operation                     | `Map`            | `BTreeMap`   |
//! |-------------------------------|------------------|--------------|
//! | `get` / `contains_key`        | `O(log n)`, binary search over one contiguous key slice | `O(log n)` |
//! | `insert` / `remove`           | `O(n)` (shifts the tail) | `O(log n)` |
//! | `collect` / `from` / `extend` | `O(n log n)` (one sort)  | `O(n log n)` |
//! | `iter` / `keys` / `values`    | slice walks      | tree walks   |
//!
//! Build a large map in one go (`collect`, `from`, `extend`) rather than
//! one `insert` at a time.

use std::borrow::Borrow;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::iter::Zip;
use std::ops::Index;
use std::{slice, vec};

/// A sorted, `Vec`-backed map with `BTreeMap`'s semantics — see the
/// [module doc](self).
///
/// Invariant: `keys` is strictly ascending and `keys.len() ==
/// values.len()`; `values[i]` belongs to `keys[i]`.
#[derive(Clone, PartialEq, Eq)]
pub struct Map<K, V> {
    pub keys: Vec<K>,
    pub values: Vec<V>,
}

impl<K, V> Map<K, V> {
    pub const fn new() -> Self {
        Map {
            keys: Vec::new(),
            values: Vec::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Map {
            keys: Vec::with_capacity(capacity),
            values: Vec::with_capacity(capacity),
        }
    }

    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    pub fn clear(&mut self) {
        self.keys.clear();
        self.values.clear();
    }

    /// Every key, ascending, as one slice.
    pub fn key_slice(&self) -> &[K] {
        &self.keys
    }

    /// Every value, in key order, as one slice.
    pub fn value_slice(&self) -> &[V] {
        &self.values
    }

    /// Every value, in key order, as one mutable slice.
    pub fn value_slice_mut(&mut self) -> &mut [V] {
        &mut self.values
    }

    /// The keys and values, ascending by key.
    pub fn into_parts(self) -> (Vec<K>, Vec<V>) {
        (self.keys, self.values)
    }

    /// Every key, ascending.
    pub fn keys(&self) -> slice::Iter<'_, K> {
        self.keys.iter()
    }

    /// Every value, in key order.
    pub fn values(&self) -> slice::Iter<'_, V> {
        self.values.iter()
    }

    pub fn values_mut(&mut self) -> slice::IterMut<'_, V> {
        self.values.iter_mut()
    }

    pub fn into_keys(self) -> vec::IntoIter<K> {
        self.keys.into_iter()
    }

    pub fn into_values(self) -> vec::IntoIter<V> {
        self.values.into_iter()
    }

    /// Every entry, ascending by key.
    pub fn iter(&self) -> Iter<'_, K, V> {
        self.keys.iter().zip(self.values.iter())
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, K, V> {
        self.keys.iter().zip(self.values.iter_mut())
    }

    pub fn first_key_value(&self) -> Option<(&K, &V)> {
        Some((self.keys.first()?, self.values.first()?))
    }

    pub fn last_key_value(&self) -> Option<(&K, &V)> {
        Some((self.keys.last()?, self.values.last()?))
    }

    /// Removes the entry with the smallest key. `O(n)`.
    pub fn pop_first(&mut self) -> Option<(K, V)> {
        if self.is_empty() {
            return None;
        }
        Some((self.keys.remove(0), self.values.remove(0)))
    }

    /// Removes the entry with the largest key. `O(1)`.
    pub fn pop_last(&mut self) -> Option<(K, V)> {
        Some((self.keys.pop()?, self.values.pop()?))
    }

    /// Keeps only the entries `f` returns `true` for, in one pass.
    pub fn retain(&mut self, mut f: impl FnMut(&K, &mut V) -> bool) {
        let mut keep = Vec::with_capacity(self.len());
        for (k, v) in self.keys.iter().zip(self.values.iter_mut()) {
            keep.push(f(k, v));
        }
        let mut flags = keep.iter();
        self.keys.retain(|_| *flags.next().unwrap());
        let mut flags = keep.iter();
        self.values.retain(|_| *flags.next().unwrap());
    }
}

impl<K: Ord, V> Map<K, V> {
    /// Where `key` is, or where it would go.
    fn search<Q>(&self, key: &Q) -> Result<usize, usize>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.keys.binary_search_by(|k| k.borrow().cmp(key))
    }

    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.search(key).ok().map(|i| &self.values[i])
    }

    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.search(key).ok().map(|i| &mut self.values[i])
    }

    pub fn get_key_value<Q>(&self, key: &Q) -> Option<(&K, &V)>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.search(key)
            .ok()
            .map(|i| (&self.keys[i], &self.values[i]))
    }

    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.search(key).is_ok()
    }

    /// Inserts `value` at `key`, returning the value it replaced. `O(n)`
    /// for a new key (the tail shifts); `O(log n)` to replace one.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        match self.search(&key) {
            Ok(i) => Some(std::mem::replace(&mut self.values[i], value)),
            Err(i) => {
                self.keys.insert(i, key);
                self.values.insert(i, value);
                None
            }
        }
    }

    /// `O(n)`: the tail shifts.
    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.remove_entry(key).map(|(_, v)| v)
    }

    pub fn remove_entry<Q>(&mut self, key: &Q) -> Option<(K, V)>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        let i = self.search(key).ok()?;
        Some((self.keys.remove(i), self.values.remove(i)))
    }

    /// Sorts `pairs` by key (stably) and keeps the last value of each run
    /// of equal keys, as repeated `BTreeMap::insert`s would.
    fn from_pairs(mut pairs: Vec<(K, V)>) -> Self {
        if !pairs.is_sorted_by(|a, b| a.0 < b.0) {
            pairs.sort_by(|a, b| a.0.cmp(&b.0));
        }
        let mut map = Map::with_capacity(pairs.len());
        for (k, v) in pairs {
            match map.keys.last() {
                Some(last) if *last == k => *map.values.last_mut().unwrap() = v,
                _ => {
                    map.keys.push(k);
                    map.values.push(v);
                }
            }
        }
        map
    }
}

pub type Iter<'a, K, V> = Zip<slice::Iter<'a, K>, slice::Iter<'a, V>>;
pub type IterMut<'a, K, V> = Zip<slice::Iter<'a, K>, slice::IterMut<'a, V>>;
pub type IntoIter<K, V> = Zip<vec::IntoIter<K>, vec::IntoIter<V>>;

impl<K, V> Default for Map<K, V> {
    fn default() -> Self {
        Map::new()
    }
}

impl<K: fmt::Debug, V: fmt::Debug> fmt::Debug for Map<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

/// Entry by entry, as `BTreeMap` compares.
impl<K: PartialOrd, V: PartialOrd> PartialOrd for Map<K, V> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.iter().partial_cmp(other.iter())
    }
}

impl<K: Ord, V: Ord> Ord for Map<K, V> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.iter().cmp(other.iter())
    }
}

/// Length prefix, then each entry — the same stream `BTreeMap` hashes.
impl<K: Hash, V: Hash> Hash for Map<K, V> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_usize(self.len());
        for entry in self.iter() {
            entry.hash(state);
        }
    }
}

impl<K, Q, V> Index<&Q> for Map<K, V>
where
    K: Ord + Borrow<Q>,
    Q: Ord + ?Sized,
{
    type Output = V;

    /// Panics when `key` is absent, like `BTreeMap`'s `Index`.
    fn index(&self, key: &Q) -> &V {
        self.get(key).expect("no entry found for key")
    }
}

impl<K: Ord, V> FromIterator<(K, V)> for Map<K, V> {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        Map::from_pairs(iter.into_iter().collect())
    }
}

/// New entries win over existing ones with the same key, as with
/// `BTreeMap::extend`. One sort over everything, not one insert each.
impl<K: Ord, V> Extend<(K, V)> for Map<K, V> {
    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, iter: I) {
        let old = std::mem::take(self);
        *self = old.into_iter().chain(iter).collect();
    }
}

impl<K: Ord, V, const N: usize> From<[(K, V); N]> for Map<K, V> {
    fn from(pairs: [(K, V); N]) -> Self {
        Map::from_pairs(Vec::from(pairs))
    }
}

impl<K: Ord, V> From<Vec<(K, V)>> for Map<K, V> {
    fn from(pairs: Vec<(K, V)>) -> Self {
        Map::from_pairs(pairs)
    }
}

/// Already sorted and unique: no sort.
impl<K, V> From<BTreeMap<K, V>> for Map<K, V> {
    fn from(map: BTreeMap<K, V>) -> Self {
        let (keys, values) = map.into_iter().unzip();
        Map { keys, values }
    }
}

impl<K: Ord, V> From<Map<K, V>> for BTreeMap<K, V> {
    fn from(map: Map<K, V>) -> Self {
        map.into_iter().collect()
    }
}

/// As a sequence of `(key, value)` pairs, ascending — the same shape a
/// `Vec<(K, V)>` serializes to.
impl<K: serde::Serialize, V: serde::Serialize> serde::Serialize for Map<K, V> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(self.iter())
    }
}

/// From a sequence of `(key, value)` pairs in any order; sorted on the way
/// in, a repeated key keeping its last value.
impl<'de, K, V> serde::Deserialize<'de> for Map<K, V>
where
    K: serde::Deserialize<'de> + Ord,
    V: serde::Deserialize<'de>,
{
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Vec::<(K, V)>::deserialize(deserializer).map(Map::from_pairs)
    }
}

impl<K, V> IntoIterator for Map<K, V> {
    type Item = (K, V);
    type IntoIter = IntoIter<K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.keys.into_iter().zip(self.values)
    }
}

impl<'a, K, V> IntoIterator for &'a Map<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, K, V> IntoIterator for &'a mut Map<K, V> {
    type Item = (&'a K, &'a mut V);
    type IntoIter = IterMut<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn behaves_like_a_btree_map() {
        let mut map = Map::from([(3, "c"), (1, "a"), (2, "b"), (1, "A")]);
        let btree = BTreeMap::from([(3, "c"), (1, "a"), (2, "b"), (1, "A")]);
        assert_eq!(
            map.iter().collect::<Vec<_>>(),
            btree.iter().collect::<Vec<_>>()
        );
        assert_eq!(map[&1], "A");
        assert_eq!(map.get(&4), None);

        assert_eq!(map.insert(0, "z"), None);
        assert_eq!(map.insert(2, "B"), Some("b"));
        assert_eq!(map.key_slice(), [0, 1, 2, 3]);
        assert_eq!(map.remove(&0), Some("z"));
        assert_eq!(map.first_key_value(), Some((&1, &"A")));
        assert_eq!(map.pop_last(), Some((3, "c")));

        map.extend([(5, "e"), (1, "one")]);
        assert_eq!(
            map.iter().collect::<Vec<_>>(),
            [(&1, &"one"), (&2, &"B"), (&5, &"e")]
        );
        map.retain(|k, _| *k != 2);
        assert_eq!(map.value_slice(), ["one", "e"]);

        let round: Map<_, _> = BTreeMap::from(map.clone()).into();
        assert_eq!(round, map);
        assert_eq!(format!("{map:?}"), r#"{1: "one", 5: "e"}"#);
    }

    #[test]
    fn string_keys_look_up_by_str() {
        let map: Map<String, u32> = [("b".to_owned(), 2), ("a".to_owned(), 1)].into();
        assert_eq!(map.get("a"), Some(&1));
        assert!(map.contains_key("b"));
        assert_eq!(map.keys().collect::<Vec<_>>(), ["a", "b"]);
    }

    #[test]
    fn hashes_and_orders_like_a_btree_map() {
        use std::collections::hash_map::DefaultHasher;
        let hash = |h: &dyn Fn(&mut DefaultHasher)| {
            let mut s = DefaultHasher::new();
            h(&mut s);
            s.finish()
        };
        let map = Map::from([(1u8, 2u8), (3, 4)]);
        let btree = BTreeMap::from([(1u8, 2u8), (3, 4)]);
        assert_eq!(hash(&|s| map.hash(s)), hash(&|s| btree.hash(s)));
        assert!(Map::from([(1, 2)]) < Map::from([(1, 3)]));
    }
}
