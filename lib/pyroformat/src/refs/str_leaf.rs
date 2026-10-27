//! [`StrNested`] — `Vec<String>`, `Vec<Vec<String>>`, … as one borrowed
//! byte blob plus a [`Topology`]. A bare `String` reads as a plain `&str`.
//!
//! # Layout
//!
//! Every string in the column is concatenated into `bytes`, with no
//! separators and no per-string header. The topology's *last* level holds
//! the byte boundaries — row `i` of a `StrNested<W, 1>` is
//! `bytes[leaf_range(i)]` — and any levels above it group those strings
//! into lists exactly as they do for [`super::Nested`].
//!
//! | view                    | behaves like         | `get(i)` / iter item          |
//! |-------------------------|----------------------|-------------------------------|
//! | `StrNested<W, 1>`    | `Vec<String>`        | `&'a str`                     |
//! | `StrNested<W, 2>`    | `Vec<Vec<String>>`   | `StrNested<W, 1>`          |
//! | …up to `N = 5`          |                      |                               |
//!
//! Note the off-by-one against [`super::Nested`]: a string is itself a list
//! of bytes, so `Vec<String>` needs one topology level where `Vec<u32>`
//! needs none.
//!
//! # UTF-8 is checked once, at load
//!
//! A view is only ever built by [`Topology::new_str`], which checks the
//! topology against `bytes` and every string as UTF-8 in the same pass as
//! it reads the topology. A view's strings are therefore always valid:
//! `get` / `get_str` are free (no check, no failure), and narrowing a view
//! (`get`, `window`, `split`) only ever hands out whole strings.
//!
//! ```
//! use pyroformat::refs::{StrNested, Topology};
//! use pyroformat::schema::StructureWord;
//! const R: u32 = <u32 as StructureWord>::RESET;
//!
//! // [["ab", "c"], [], ["def"]]: the two levels, back to back.
//! let words: [u32; 8] = [R, 2, 2, 3, R, 2, 3, 6];
//! let nest: StrNested<u32, 2> = Topology::new_str(0, &words, b"abcdef").unwrap();
//!
//! assert_eq!(nest.len(), 3);
//! let first = nest.get(0).unwrap();                 // StrNested<W, 1>
//! assert_eq!(first.get(1), Some("c"));
//! assert_eq!(first.get_bytes(0), Some(&b"ab"[..]));
//! assert!(nest.get(1).unwrap().is_empty());
//!
//! let last: Vec<&str> = nest.get(2).unwrap().iter().collect();
//! assert_eq!(last, ["def"]);
//!
//! // Bytes the topology doesn't account for, or bad UTF-8, never load.
//! assert!(Topology::<u32, 2>::new_str(0, &words, b"abcdefg").is_err());
//! assert!(Topology::<u32, 2>::new_str(0, &words, b"abcde\xff").is_err());
//! ```

use crate::PyroValue;
use crate::errors::WireError;
use crate::interface::{Realize, RowsRef};
use crate::refs::Topology;
use pyro_schema::StructureWord;

/// See the [module doc](self).
///
/// Invariant: every string the topology's last written level marks out of
/// `bytes` is valid UTF-8 (so `bytes` is too), and that level's last
/// boundary is `bytes.len()`. Only [`Topology::new_str`] builds one from a
/// buffer, which is why the fields are private.
#[derive(Clone, Copy)]
pub struct StrNested<'a, W: StructureWord, const N: usize> {
    topology: Topology<'a, W, N>,
    bytes: &'a [u8],
}

impl<'a, W: StructureWord, const N: usize> std::fmt::Debug for StrNested<'a, W, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StrNested")
            .field("topology", &self.topology)
            .field("bytes", &self.bytes)
            .finish()
    }
}

impl<'a, W: StructureWord, const N: usize> StrNested<'a, W, N> {
    /// A view over parts that already hold the type's invariant.
    ///
    /// # Safety
    ///
    /// Every string `topology`'s last written level marks out of `bytes`
    /// must be valid UTF-8, and that level's last boundary must be
    /// `bytes.len()` — as
    /// [`Topology::new_str`] checks. The views hand those strings out as
    /// `&str` without checking them again.
    pub(crate) unsafe fn from_validated(topology: Topology<'a, W, N>, bytes: &'a [u8]) -> Self {
        StrNested { topology, bytes }
    }

    /// Number of elements at this view's outermost level.
    pub fn len(&self) -> usize {
        self.topology.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Elements `[start, end)`, same depth, `O(1)`. `None` when out of
    /// bounds.
    pub fn window(&self, start: usize, end: usize) -> Option<Self> {
        Some(StrNested {
            topology: self.topology.window(start, end)?,
            bytes: self.bytes,
        })
    }

    /// [`Topology::split`] for a view read at a fixed depth `N` above the
    /// schema's: splits off level `levels - 1`, the strings, leaving the
    /// lists above them. Errs when the data wrote a level below it, since
    /// then level `levels - 1` isn't the one [`Topology::new_str`] checked
    /// against `bytes`.
    pub fn split(
        self,
        levels: usize,
    ) -> Result<(Topology<'a, W, N>, Option<StrNested<'a, W, 1>>), WireError> {
        if levels == 0 || (levels < N && !self.topology.topology[levels].is_empty()) {
            return Err(WireError::InvalidTopologyDepth { max: levels });
        }
        let (topology, split) = self.topology.split(levels);
        Ok((
            topology,
            split.map(|topology| StrNested {
                topology,
                bytes: self.bytes,
            }),
        ))
    }

    pub fn iter(&self) -> StrNestedIter<'a, W, N> {
        StrNestedIter {
            nest: *self,
            front: 0,
            back: self.len(),
        }
    }
}

/// The first string in the last written level of `topology` that isn't
/// valid UTF-8, as [`WireError::Utf8`] with `range` the offending bytes'
/// position within `bytes` — strings checked one by one, so a char split
/// across two strings is an error. Only [`Topology::new_str`]'s error path
/// calls this, to describe a rejection; `N = 0` checks `bytes` whole.
pub(crate) fn check_strs<W: StructureWord, const N: usize>(
    offset: usize,
    topology: &Topology<'_, W, N>,
    bytes: &[u8],
) -> Result<(), WireError> {
    if N == 0 {
        return check_utf8(bytes, 0);
    }
    // `N` only bounds the depth: writing stops at the first level that ends
    // in 0, so the last written level can sit above `N - 1`; no level at
    // all means no strings.
    let Some(last) = topology.depth().checked_sub(1) else {
        return Ok(());
    };
    let mut start = 0;
    for w in &topology.topology[last][1..] {
        let end = w.to_usize();
        let s =
            bytes
                .get(start..end)
                .ok_or(WireError::RawSliceOutOfRange(start, end, bytes.len()))?;
        check_utf8(s, start + offset)?;
        start = end;
    }
    Ok(())
}

/// A bare string's bytes as a `&str`, or [`WireError::Utf8`] with the bad
/// bytes' range shifted by `offset` (the string's position in the message).
#[inline]
pub(crate) fn check_str(offset: usize, bytes: &[u8]) -> Result<&str, WireError> {
    check_utf8(bytes, offset)?;
    // SAFETY: just checked.
    Ok(unsafe { std::str::from_utf8_unchecked(bytes) })
}

/// `bytes` as UTF-8, or the failure with its range shifted by `base`.
#[inline]
fn check_utf8(bytes: &[u8], base: usize) -> Result<(), WireError> {
    match std::str::from_utf8(bytes) {
        Ok(_) => Ok(()),
        Err(source) => {
            let start = base + source.valid_up_to();
            let end = match source.error_len() {
                Some(n) => start + n,
                None => base + bytes.len(),
            };
            Err(WireError::Utf8 {
                range: start..end,
                source,
            })
        }
    }
}

impl<'a, W: StructureWord> StrNested<'a, W, 1> {
    /// String `i`'s bytes. `None` when out of bounds.
    pub fn get_bytes(&self, i: usize) -> Option<&'a [u8]> {
        let (start, end) = self.topology.leaf_range(i)?;
        self.bytes.get(start..end)
    }

    /// String `i` — no check, it was validated at load. `None` when out
    /// of bounds.
    pub fn get(&self, i: usize) -> Option<&'a str> {
        // At `N = 1` the only level is the last written one, so string `i`
        // is one of the strings the invariant covers.
        let bytes = self.get_bytes(i)?;
        // SAFETY: UTF-8 by the type's invariant.
        Some(unsafe { std::str::from_utf8_unchecked(bytes) })
    }

    /// Alias for [`Self::get`].
    pub fn get_str(&self, i: usize) -> Option<&'a str> {
        self.get(i)
    }

    /// Alias for [`Self::get`].
    pub fn descend(&self, i: usize) -> Option<&'a str> {
        self.get(i)
    }
}

macro_rules! str_nest_level {
    ($n:literal => $m:literal) => {
        impl<'a, W: StructureWord> StrNested<'a, W, $n> {
            /// Element `i`, one level down.
            pub fn get(&self, i: usize) -> Option<StrNested<'a, W, $m>> {
                Some(StrNested {
                    topology: self.topology.descend(i)?,
                    bytes: self.bytes,
                })
            }

            /// Alias for [`Self::get`].
            pub fn descend(&self, i: usize) -> Option<StrNested<'a, W, $m>> {
                self.get(i)
            }
        }
    };
}

str_nest_level!(2 => 1);
str_nest_level!(3 => 2);
str_nest_level!(4 => 3);
str_nest_level!(5 => 4);

/// Iterator over a [`StrNested`]'s outermost level; yields what
/// `get(i)` would, front to back (or back to front).
pub struct StrNestedIter<'a, W: StructureWord, const N: usize> {
    nest: StrNested<'a, W, N>,
    front: usize,
    back: usize,
}

macro_rules! str_nest_iter {
    ($n:literal => $item:ty) => {
        impl<'a, W: StructureWord> Iterator for StrNestedIter<'a, W, $n> {
            type Item = $item;

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
                self.nest.get(i)
            }

            fn size_hint(&self) -> (usize, Option<usize>) {
                let n = self.back - self.front;
                (n, Some(n))
            }
        }

        impl<'a, W: StructureWord> DoubleEndedIterator for StrNestedIter<'a, W, $n> {
            fn next_back(&mut self) -> Option<Self::Item> {
                if self.front >= self.back {
                    return None;
                }
                self.back -= 1;
                self.nest.get(self.back)
            }
        }

        impl<'a, W: StructureWord> ExactSizeIterator for StrNestedIter<'a, W, $n> {}

        impl<'a, W: StructureWord> IntoIterator for StrNested<'a, W, $n> {
            type Item = $item;
            type IntoIter = StrNestedIter<'a, W, $n>;

            fn into_iter(self) -> Self::IntoIter {
                self.iter()
            }
        }
    };
}

str_nest_iter!(1 => &'a str);
str_nest_iter!(2 => StrNested<'a, W, 1>);
str_nest_iter!(3 => StrNested<'a, W, 2>);
str_nest_iter!(4 => StrNested<'a, W, 3>);
str_nest_iter!(5 => StrNested<'a, W, 4>);

// =============================================================================
// RowsRef — forwards to each level's own `get` (not `descend`)
// =============================================================================

macro_rules! str_nest_rows_ref {
    ($($n:literal => $row:ty),* $(,)?) => {
        $(
            impl<'a, W: StructureWord> RowsRef for StrNested<'a, W, $n> {
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
        )*
    };
}

str_nest_rows_ref!(
    1 => &'a str,
    2 => StrNested<'a, W, 1>,
    3 => StrNested<'a, W, 2>,
    4 => StrNested<'a, W, 3>,
    5 => StrNested<'a, W, 4>,
);

// =============================================================================
// Realize
// =============================================================================

impl<'a, W: StructureWord, const N: usize> StrNested<'a, W, N> {
    /// The elements of this view's outermost level as nested
    /// `PyroValue::List`s of borrowed `PyroValue::Str`s. The last level in
    /// use holds the string boundaries, so `levels == 1` yields the strings
    /// themselves.
    ///
    /// `levels` is how many of the view's `N` levels the data uses
    /// (`1 <= levels <= N`): a view read at [`crate::MAX_TIERS`] levels for a
    /// schema only known at runtime uses fewer.
    pub fn into_value(&self, levels: usize) -> Vec<PyroValue<'a>> {
        let bytes = self.bytes;
        self.topology.fold_rows(levels, |leaf| {
            StrNested::<'a, W, 1> {
                topology: leaf,
                bytes,
            }
            .iter()
            .map(PyroValue::from)
        })
    }
}

impl<'a, W: StructureWord> Realize<Vec<String>> for StrNested<'a, W, 1> {
    fn realize(&self) -> Vec<String> {
        self.iter().map(str::to_owned).collect()
    }
}

macro_rules! str_nest_realize {
    ($($n:literal => $m:literal),* $(,)?) => {
        $(
            impl<'a, W: StructureWord, O> Realize<Vec<O>> for StrNested<'a, W, $n>
            where
                StrNested<'a, W, $m>: Realize<O>,
            {
                fn realize(&self) -> Vec<O> {
                    self.iter().map(|child| child.realize()).collect()
                }
            }
        )*
    };
}

str_nest_realize!(2 => 1, 3 => 2, 4 => 3, 5 => 4);

// =============================================================================
// Equality — compares like the `Vec<…<String>…>` it stands for, against
// another view or against `Vec`s of `String`s / `&str`s (both directions)
// =============================================================================

macro_rules! str_nest_eq {
    ($($n:literal => $child:ty),* $(,)?) => {
        $(
            impl<'a, 'b, W: StructureWord> PartialEq<StrNested<'b, W, $n>> for StrNested<'a, W, $n> {
                fn eq(&self, other: &StrNested<'b, W, $n>) -> bool {
                    self.len() == other.len()
                        && (0..self.len()).all(|i| self.descend(i) == other.descend(i))
                }
            }

            impl<'a, W: StructureWord> Eq for StrNested<'a, W, $n> {}

            impl<'a, W: StructureWord, V> PartialEq<[V]> for StrNested<'a, W, $n>
            where
                $child: PartialEq<V>,
            {
                fn eq(&self, other: &[V]) -> bool {
                    self.len() == other.len()
                        && other
                            .iter()
                            .enumerate()
                            .all(|(i, b)| self.descend(i).is_some_and(|a| a == *b))
                }
            }

            impl<'a, W: StructureWord, V> PartialEq<Vec<V>> for StrNested<'a, W, $n>
            where
                $child: PartialEq<V>,
            {
                fn eq(&self, other: &Vec<V>) -> bool {
                    *self == *other.as_slice()
                }
            }

            impl<'a, W: StructureWord, V> PartialEq<StrNested<'a, W, $n>> for Vec<V>
            where
                $child: PartialEq<V>,
            {
                fn eq(&self, other: &StrNested<'a, W, $n>) -> bool {
                    *other == *self.as_slice()
                }
            }
        )*
    };
}

str_nest_eq!(
    1 => &'a str,
    2 => StrNested<'a, W, 1>,
    3 => StrNested<'a, W, 2>,
    4 => StrNested<'a, W, 3>,
    5 => StrNested<'a, W, 4>,
);

#[cfg(test)]
mod tests {
    use super::*;
    use pyro_schema::StructureWord;

    const R: u32 = <u32 as StructureWord>::RESET;

    #[test]
    fn deep_levels_and_windows() {
        // [[["a"], ["bc", ""]], [], [["d"]]]
        let l0 = [R, 2, 2, 3];
        let l1 = [R, 1, 3, 4];
        let l2 = [R, 1, 3, 3, 4];
        let nest: StrNested<'_, u32, 3> = StrNested {
            topology: Topology {
                topology: [&l0, &l1, &l2],
            },
            bytes: b"abcd",
        };
        let collect =
            |n: StrNested<'_, u32, 1>| -> Vec<String> { n.iter().map(str::to_owned).collect() };
        let got: Vec<Vec<Vec<String>>> = nest
            .iter()
            .map(|lists| lists.iter().map(collect).collect())
            .collect();
        assert_eq!(
            got,
            vec![
                vec![vec!["a".to_owned()], vec!["bc".to_owned(), "".to_owned()]],
                vec![],
                vec![vec!["d".to_owned()]],
            ]
        );
        assert_eq!(nest.iter().rev().next().unwrap().len(), 1);
        let tail = nest.window(1, 3).unwrap();
        assert_eq!(tail.len(), 2);
        assert_eq!(
            tail.get(1).unwrap().get(0).unwrap().get_bytes(0),
            Some(&b"d"[..])
        );
        assert!(nest.get(3).is_none());
    }

    #[test]
    fn omitted_lower_level() {
        // [[], []]: the string level is never written.
        let l0 = [R, 0, 0];
        let nest: StrNested<'_, u32, 2> = StrNested {
            topology: Topology {
                topology: [&l0, &[]],
            },
            bytes: &[],
        };
        assert_eq!(nest.iter().count(), 2);
        assert!(nest.iter().all(|list| list.is_empty()));
    }

    #[test]
    fn check_strs_reports_the_bad_string() {
        let strs = [R, 1, 3];
        let topology = Topology {
            topology: [&strs[..]],
        };
        match check_strs(0, &topology, b"a\xffb") {
            Err(WireError::Utf8 { range, .. }) => assert_eq!(range, 1..2),
            other => panic!("{other:?}"),
        }
        // "é" is two bytes; a boundary between them splits it.
        let split = [R, 1, 2];
        let topology = Topology {
            topology: [&split[..]],
        };
        assert!(check_strs(0, &topology, "é".as_bytes()).is_err());
        assert!(check_strs::<u32, 0>(0, &Topology { topology: [] }, b"ok").is_ok());
    }

    #[test]
    fn equality_matches_owned_strings() {
        // [["ab", "c"], [], ["def"]]
        let lists = [R, 2, 2, 3];
        let strs = [R, 2, 3, 6];
        let nest: StrNested<'_, u32, 2> = StrNested {
            topology: Topology {
                topology: [&lists, &strs],
            },
            bytes: b"abcdef",
        };
        let owned: Vec<Vec<String>> =
            vec![vec!["ab".into(), "c".into()], vec![], vec!["def".into()]];
        assert_eq!(nest, owned);
        assert_eq!(owned, nest);
        assert_eq!(nest.get(0).unwrap(), vec!["ab", "c"]);
        assert_eq!(nest.get(0).unwrap().descend(1).unwrap(), "c");
        assert_eq!(nest, nest.window(0, 3).unwrap());
        assert_ne!(nest.get(0).unwrap(), nest.get(2).unwrap());
        assert_ne!(nest, vec![vec!["ab".to_owned()]]);
    }
}
