use pyro_schema::StructureWord;

use crate::errors::WireError;
use crate::refs::{StrNested, check_strs};
use crate::{MAX_TIERS, PyroValue};

/// One nested list's topology, on the value path: every level's
/// sentinel-prefixed cumulative-count array, borrowed from the buffer.
///
/// `N` is the levels the view holds. A message has as many levels as its
/// inode's nesting depth, except that writing stops at the first level that
/// ends in 0 (nothing is ever ingested below it), so the remaining slots of
/// `topology` are just `&[]` rather than an error. A level ending in anything
/// else must have the next one behind it.
///
/// [`Self::new`] loads a static view, whose `N` is the inode's depth. The
/// value path reads every inode into a [`MAX_TIERS`] view instead, and
/// passes the inode's depth to [`Self::new_dynamic`].
///
#[derive(Debug, Clone, Copy)]
pub struct Topology<'a, W: StructureWord, const N: usize> {
    pub topology: [&'a [W]; N],
}

impl<'a, W: StructureWord, const N: usize> Topology<'a, W, N> {
    /// Resolve every level from the `(start, count)` slot at
    /// `topology_offset`, two `W`s: `start` is the byte offset of the
    /// topology's own `W` words, `count` is how many of them there are
    /// (sentinels included).
    ///
    /// The view's `N` is the depth: every level that is written must be
    /// there, up to `N`.
    #[inline]
    pub fn new(words: &'a [W]) -> Result<Self, WireError> {
        Self::load::<false>(0, words, &[], N)
    }

    /// [`Self::new`] for a string column, checking `bytes` (every string,
    /// concatenated) against the topology in the same pass: the last level's
    /// final boundary must be `bytes.len()`, and every string must be UTF-8.
    ///
    /// `bytes` is checked as UTF-8 once, whole, and every boundary must fall
    /// on a char boundary — together the same as checking each string on
    /// its own (a char split across two strings is an error), without one
    /// `from_utf8` call per string. On failure the error is the first bad
    /// string's, with its byte range within `bytes`.
    ///
    /// This and [`Topology::new_str_dynamic`] are the only ways to load a
    /// [`StrNested`]: the view relies on these checks and never repeats them.
    #[inline]
    pub fn new_str(
        offset: usize,
        words: &'a [W],
        bytes: &'a [u8],
    ) -> Result<StrNested<'a, W, N>, WireError> {
        Self::load_str(offset, words, bytes, N)
    }
}

/// The value path's views: every inode is read into a [`MAX_TIERS`]-level
/// view, so its depth is only known at runtime and passed in.
impl<'a, W: StructureWord> Topology<'a, W, MAX_TIERS> {
    /// [`Topology::new`] for a topology `levels` deep (at most
    /// [`MAX_TIERS`]): every level that is written must be there, up to
    /// `levels`.
    #[inline]
    pub fn new_dynamic(words: &'a [W], levels: usize) -> Result<Self, WireError> {
        Self::load::<false>(0, words, &[], levels)
    }

    /// [`Topology::new_str`] for a topology `levels` deep (at most
    /// [`MAX_TIERS`]).
    #[inline]
    pub fn new_str_dynamic(
        offset: usize,
        words: &'a [W],
        bytes: &'a [u8],
        levels: usize,
    ) -> Result<StrNested<'a, W, MAX_TIERS>, WireError> {
        Self::load_str(offset, words, bytes, levels)
    }
}

impl<'a, W: StructureWord, const N: usize> Topology<'a, W, N> {
    /// [`Self::new_str`] for a topology `levels` deep (at most `N`).
    #[inline(always)]
    fn load_str(
        offset: usize,
        words: &'a [W],
        bytes: &'a [u8],
        levels: usize,
    ) -> Result<StrNested<'a, W, N>, WireError> {
        let topology = if levels == 0 {
            // One bare string: there is no topology to read.
            let topology = Topology { topology: [&[]; N] };
            match std::str::from_utf8(bytes) {
                Ok(_) => {}
                Err(source) => {
                    let start = offset + source.valid_up_to();
                    let end = match source.error_len() {
                        Some(n) => start + n,
                        None => offset + bytes.len(),
                    };
                    return Err(WireError::Utf8 {
                        range: start..end,
                        source,
                    });
                }
            }
            topology
        } else {
            Self::load::<true>(offset, words, bytes, levels)?
        };
        // SAFETY: `load::<true>` (or the check above, at no levels) has pinned
        // the last level's final boundary to `bytes.len()` and checked every
        // string it marks out as UTF-8.
        Ok(unsafe { StrNested::from_validated(topology, bytes) })
    }

    /// [`Self::new`]'s loop, for a topology `levels` deep (at most `N`);
    /// with `STR`, also [`Self::new_str`]'s checks of `bytes` against the
    /// last level. `STR` is a const so `new` compiles without any of them.
    #[inline(always)]
    fn load<const STR: bool>(
        offset: usize,
        words: &'a [W],
        bytes: &'a [u8],
        levels: usize,
    ) -> Result<Self, WireError> {
        debug_assert!(levels <= N, "{levels} levels in a {N}-level view");
        if words[0] != W::RESET {
            return Err(WireError::InvalidTopologySentinel {
                word: 0,
                found: words[0].to_usize(),
            });
        }
        let mut topology: [&'a [W]; N] = [&[]; N];

        // Levels are written outermost-first (see `TopologyCache::write`).
        // Only level 0's length has to be searched for; every level below
        // holds exactly as many boundaries as the last boundary of the level
        // above, so its end is known up front and just has to land on the
        // next sentinel (or the end of the words).
        let mut len = words[1..]
            .iter()
            .position(|&w| w == W::RESET)
            .unwrap_or(words.len() - 1);
        let mut at = 0usize;
        let mut level = 0usize;
        loop {
            if level == levels {
                return Err(WireError::InvalidTopologyDepth { max: levels });
            }
            // A sentinel straight after a sentinel: for level 0 that's
            // `position` hitting index 0, below it the level above ending in
            // 0 with words still to come — writing stops before either.
            if len == 0 {
                return Err(WireError::InvalidTopologyEmptyLevel { level, word: at });
            }
            let end = at + 1 + len;
            let fits = end <= words.len() && (end == words.len() || words[end] == W::RESET);
            if !fits {
                return Err(level_error(level, len, &words[at..]));
            }
            let bounds = &words[at + 1..end];
            let last = bounds[len - 1];
            let is_final = end == words.len();
            // The last level's boundaries index `bytes`; fold the char
            // boundary checks into the same pass as the order check.
            let (ordered, on_chars) = if STR && is_final {
                monotonic_on_chars(bounds, bytes)
            } else {
                (monotonic(bounds), true)
            };
            // `RESET` is `W::MAX`, so a sentinel inside the level drags every
            // boundary after it up to `RESET` too — checking the last one
            // catches it without a per-word compare.
            if last == W::RESET || !ordered {
                return Err(level_error(level, len, &words[at..]));
            }
            let last = last.to_usize();
            topology[level] = &words[at..end];
            if is_final {
                // Writing only stops early below a level that ends in 0: a
                // level still pointing at elements one level down needs that
                // level behind it, or `total` would read this level's list
                // count as the leaf count.
                if level + 1 < levels && last != 0 {
                    return Err(WireError::InvalidTopologyLevel {
                        level,
                        last,
                        next: 0,
                    });
                }
                if STR {
                    if last != bytes.len() {
                        return Err(WireError::InvalidTopologyLen {
                            topo: last,
                            data: bytes.len(),
                        });
                    }
                    if !on_chars || std::str::from_utf8(bytes).is_err() {
                        check_strs(offset, &Topology { topology }, bytes)?;
                    }
                }
                return Ok(Topology { topology });
            }
            at = end;
            len = last;
            level += 1;
        }
    }

    /// The final element in the final non-zero row has the total number of elements
    pub fn total(&self) -> usize {
        // `N..0` is an empty range — walk the levels deepest-first.
        for i in (0..N).rev() {
            if let Some(&n) = self.topology[i].last() {
                if n == W::RESET {
                    return 0;
                }
                return n.to_usize();
            }
        }
        0
    }

    pub fn is_terminal(&self) -> bool {
        N <= 1 || self.topology[1].is_empty()
    }

    pub fn depth(&self) -> usize {
        for i in 0..N {
            if self.topology[i].is_empty() {
                return i;
            }
        }
        N
    }
    // Splits the final topology off
    /// Split off the leaf-adjacent level, `levels - 1` (the schema's topology
    /// level count), if it was written. If writing stopped above it, every
    /// element on the last written level is an empty list of lists, and the
    /// leaf nest stays empty.
    pub fn split(mut self, levels: usize) -> (Self, Option<Topology<'a, W, 1>>) {
        if self.topology[0].is_empty() {
            return (self, None);
        }
        let mut split = Topology { topology: [&[]; 1] };
        let leaf = levels - 1;
        if leaf < N && !self.topology[leaf].is_empty() {
            split.topology[0] = self.topology[leaf];
            self.topology[leaf] = &[];
        }
        (self, Some(split))
    }

    /// Narrow to element `index`'s own range in the level below this one.
    /// `None` once there is nothing to descend into — either `index` is out
    /// of range, or this view's last level is already the leaf level.
    pub fn get(&self, index: usize) -> Option<Self> {
        let start = if *self.topology[0].get(index)? == W::RESET {
            0
        } else {
            self.topology[0].get(index)?.to_usize()
        };
        let end = self.topology[0].get(index + 1)?.to_usize();
        let mut topology: [&[W]; N] = [&[]; N];
        topology[0] = self.topology[1].get(start..=end)?;
        for i in 2..N {
            topology[i - 1] = self.topology[i];
        }

        Some(Topology { topology })
    }

    /// Narrow to element `index`'s own range in the level below this one.
    /// `None` once there is nothing to descend into — either `index` is out
    /// of range, or this view's last level is already the leaf level.
    pub fn range(&self, start: usize, end: usize) -> Option<Self> {
        let mut topology: [&[W]; N] = [&[]; N];
        topology[0] = self.topology[0].get(start..=end)?;
        for i in 1..N {
            topology[i] = self.topology[i];
        }

        Some(Topology { topology })
    }

    /// An all-empty topology: zero elements at every level.
    pub fn empty() -> Self {
        Topology { topology: [&[]; N] }
    }

    /// Elements `[start, end)` of this view's own level, same depth,
    /// `O(1)`. `None` when out of bounds. Unlike [`Self::range`], an empty
    /// window is always fine — even over a level with no boundary words at
    /// all, which [`TopologyCache::write`] produces for an all-empty level.
    pub fn window(&self, start: usize, end: usize) -> Option<Self> {
        if start > end || end > self.len() {
            None
        } else if start == end {
            Some(Self::empty())
        } else {
            self.range(start, end)
        }
    }

    /// The elements of this topology's outermost level when the data uses
    /// its first `levels` levels (`1 <= levels <= N`; a view read at
    /// [`crate::MAX_TIERS`] levels for a schema only known at runtime uses
    /// fewer). `rows` builds the leaf rows from level `levels - 1`, split
    /// off as a one-level topology: a `&[T]` per list for a primitive
    /// column, an `OptionSlice` per list for a nullable one, …; the levels
    /// above it fold them up with [`Self::eat_values`].
    ///
    /// Every view's constructor checks its topology against its data, so
    /// the rows always cover the leaf level.
    pub(crate) fn fold_rows<'b, I>(
        self,
        levels: usize,
        rows: impl FnOnce(Topology<'a, W, 1>) -> I,
    ) -> Vec<PyroValue<'b>>
    where
        I: Iterator<Item = PyroValue<'b>>,
    {
        debug_assert!(
            (1..=N).contains(&levels),
            "{levels} levels in a {N}-level view"
        );
        if levels == 0 || levels > N {
            return Vec::new();
        }
        let (upper, Some(leaf)) = self.split(levels) else {
            return Vec::new();
        };
        let mut rows = rows(leaf);
        if levels == 1 {
            return rows.collect();
        }
        upper
            .eat_values(&mut rows)
            .expect("a view's topology is checked against its rows at load")
    }

    /// Depth first consumption of data (slices, bytes). Depth first for efficient allocations
    ///
    /// Consume can be: |start: usize, end: usize, slice: &mut [f32]| -> {
    ///     Ok(PyroValue::PrimitiveList(&slice[start..end].into()))
    /// }
    ///
    /// Or with strings it should be |start: usize, end: usize, slice: &mut [u8]| -> {
    ///     std::str::from_utf8(&slice[start..end]).map_err(|_| WireError::invalid_utf8())
    /// }
    ///
    /// D can be a compound type. For example if we're reading a group value it will be [PyroValue<'a>>].
    /// in which case we want to take the values from the vec and replace them with Nulls.
    ///
    /// |start: usize, end: usize, slice: &mut [PyroValue<'a>>]| -> {
    ///     let list = Vec::with_capacity(end - start);
    ///     for v in slice[start..end] {
    ///         let mut m = PyroValue::Null;
    ///         std::mem::replace(v, m);
    ///         list.push(m);
    ///     }
    ///     Ok(PyroValue::List(list))
    /// }
    pub fn eat_values<'b, E, D: Iterator<Item = E>>(
        &self,
        data: &mut D,
    ) -> Result<Vec<PyroValue<'b>>, WireError>
    where
        PyroValue<'b>: From<E>,
    {
        struct Frame<'b> {
            next: usize,
            results: Vec<PyroValue<'b>>,
        }

        const FRAME_INIT: Frame<'static> = Frame {
            next: 0,
            results: Vec::new(),
        };

        // SAFETY: every `Frame` starts with an empty `results`, which holds
        // no `'static` data to begin with — narrowing that lifetime to `'b`
        // changes nothing about what's actually there.
        let mut frames =
            unsafe { std::mem::transmute::<[Frame<'static>; N], [Frame<'b>; N]>([FRAME_INIT; N]) };

        let empty_topology = Topology {
            topology: [<&[W]>::default(); N],
        };
        let mut topologies: [Topology<'a, W, N>; N] = [empty_topology; N];

        topologies[0] = Topology {
            topology: self.topology,
        };
        frames[0].results = Vec::with_capacity(self.len());
        let mut top = 1usize;

        loop {
            let len = topologies[top - 1].len();

            if frames[top - 1].next >= len {
                let finished = std::mem::take(&mut frames[top - 1].results);
                top -= 1;
                if top == 0 {
                    return Ok(finished);
                }
                frames[top - 1].results.push(PyroValue::List(finished));
                continue;
            }

            let idx = frames[top - 1].next;
            frames[top - 1].next += 1;

            if topologies[top - 1].is_terminal() {
                let (start, end) = topologies[top - 1]
                    .leaf_range(idx)
                    .expect("idx < len, checked above");
                let values = data
                    .take(end - start)
                    .map(|v| PyroValue::from(v))
                    .collect::<Vec<_>>();
                if values.len() < end - start {
                    return Err(WireError::InvalidTopologyLen {
                        topo: end - start,
                        data: values.len(),
                    });
                }
                frames[top - 1].results.push(PyroValue::List(values));
            } else {
                match topologies[top - 1].get(idx) {
                    Some(child) => {
                        frames[top] = Frame {
                            next: 0,
                            results: Vec::with_capacity(child.len()),
                        };
                        topologies[top] = child;
                        top += 1;
                    }
                    // The level below is entirely absent (never ingested,
                    // per `TopologyCache::write`'s "omit empty levels"
                    // rule) — a real empty sublist, not an error.
                    None => frames[top - 1].results.push(PyroValue::List(Vec::new())),
                }
            }
        }
    }

    /// [`Self::fold_rows`] for a compound leaf (a `Map`'s entries, or
    /// already-built `Group`/`Option<Group>` values) that has no byte range
    /// of its own, only a *count* per leaf-level index of how many
    /// sequential items `data` contributes to it.
    ///
    /// `levels` is the schema's level count for this view (`1 <= levels <=
    /// N`). The leaf level, `levels - 1`, is split off by that count, not
    /// found by looking for the first empty level: writing stops at the
    /// first level that ends in 0, so a level missing below the last one
    /// written means every list there is empty, not that the leaf level
    /// moved up. Each leaf-level index pulls its `(end - start)` items off
    /// `data` in order and hands them to `consume` once, which folds them
    /// into the one compound value that index stands for (e.g.
    /// `PyroValue::Map`). The levels above fold those up with
    /// [`Self::eat_values`].
    pub fn eat_compound_values<'b, E, F, D: Iterator<Item = E>>(
        self,
        levels: usize,
        mut data: D,
        consume: F,
    ) -> Result<Vec<PyroValue<'b>>, WireError>
    where
        F: Fn(Vec<E>) -> PyroValue<'b>,
    {
        debug_assert!(
            (1..=N).contains(&levels),
            "{levels} levels in a {N}-level view"
        );
        if levels == 0 || levels > N {
            return Ok(Vec::new());
        }
        let (upper, Some(leaf)) = self.split(levels) else {
            return Ok(Vec::new());
        };
        let mut rows = Vec::with_capacity(leaf.len());
        for idx in 0..leaf.len() {
            let (start, end) = leaf.leaf_range(idx).expect("idx < len");
            let mut items = Vec::with_capacity(end - start);
            for _ in start..end {
                items.push(data.next().ok_or(WireError::InvalidInodes)?);
            }
            rows.push(consume(items));
        }
        if levels == 1 {
            return Ok(rows);
        }
        upper.eat_values(&mut rows.into_iter())
    }

    /// Element count of this view's own level.
    pub fn len(&self) -> usize {
        self.topology[0].len().saturating_sub(1)
    }

    /// Element `index`'s own `(start, end)` range at this view's level —
    /// the same boundary lookup [`Self::descend`] does on `self.topology[0]`
    /// to find where to narrow, exposed here for a caller (like
    /// [`Self::eat_values`]) that wants the range itself rather than a
    /// narrowed [`Topology`].
    pub fn leaf_range(&self, index: usize) -> Option<(usize, usize)> {
        let start = if *self.topology[0].get(index)? == W::RESET {
            0
        } else {
            self.topology[0].get(index)?.to_usize()
        };
        let end = self.topology[0].get(index + 1)?.to_usize();
        Some((start, end))
    }

    /// Checks a whole topology, as [`Self::new`] reads it, against the list
    /// it indexes, which holds `expected_len` elements:
    ///
    /// * every level starts with its sentinel, and its boundaries never
    ///   decrease;
    /// * every level's final boundary — its total count one level down — is
    ///   the number of lists the next level holds;
    /// * the last level's final boundary is `expected_len`.
    ///
    /// "The last level" is the last one written, which can come before level
    /// `N - 1`: `N` is only a bound, and writing stops at the first level
    /// that ends in 0 (so `expected_len` must then be 0). A topology with no
    /// levels at all indexes an empty list.
    ///
    /// A view narrowed by [`Self::get`], [`Self::range`] or [`Self::window`]
    /// keeps its deeper levels whole, so it doesn't satisfy these; validate
    /// before narrowing.
    pub fn validate(&self, expected_len: usize) -> Result<(), WireError> {
        let depth = self.depth();
        if depth == 0 {
            return if expected_len == 0 {
                Ok(())
            } else {
                Err(WireError::InvalidTopologyLen {
                    topo: 0,
                    data: expected_len,
                })
            };
        }

        // Where each level starts within the topology's word run, for the
        // sentinel error, which reports positions in that run.
        let mut word = 0usize;
        for level in 0..depth {
            let words = self.topology[level];
            if words[0] != W::RESET {
                return Err(WireError::InvalidTopologySentinel {
                    word,
                    found: words[0].to_usize(),
                });
            }
            let mut last = 0usize;
            for (index, boundary) in words.iter().enumerate().skip(1) {
                let boundary = boundary.to_usize();
                if boundary < last {
                    return Err(WireError::InvalidTopologyOrder {
                        level,
                        index,
                        previous: last,
                        found: boundary,
                    });
                }
                last = boundary;
            }

            if level + 1 < depth {
                let next = self.topology[level + 1].len() - 1;
                if last != next {
                    return Err(WireError::InvalidTopologyLevel { level, last, next });
                }
            } else if last != expected_len {
                return Err(WireError::InvalidTopologyLen {
                    topo: last,
                    data: expected_len,
                });
            }
            word += words.len();
        }
        Ok(())
    }
}

impl<W: StructureWord> Default for Topology<'_, W, 0> {
    fn default() -> Self {
        Self {
            topology: Default::default(),
        }
    }
}

pub struct TopologyIter<'a, W: StructureWord, const N: usize> {
    topology: Topology<'a, W, N>,
    index: usize,
}

impl<'a, W: StructureWord, const N: usize> Iterator for TopologyIter<'a, W, N> {
    type Item = Topology<'a, W, N>;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.topology.get(self.index);
        self.index += 1;
        next
    }
}

pub struct TopologyDecendIter<'a, W: StructureWord, const N: usize> {
    topology: Topology<'a, W, N>,
    index: usize,
}

impl<'a, W: StructureWord> Topology<'a, W, 1> {
    /// Narrow to element `index`'s own range in the level below this one.
    /// `None` once there is nothing to descend into — either `index` is out
    /// of range, or this view's last level is already the leaf level.
    pub fn descend(&self, _index: usize) -> Option<Topology<'a, W, 0>> {
        let topology: [&[W]; 0] = [&[]; 0];

        Some(Topology { topology })
    }

    pub fn descend_iter(&self) -> TopologyDecendIter<'a, W, 1> {
        TopologyDecendIter {
            topology: *self,
            index: 0,
        }
    }
}

impl<'a, W: StructureWord> Iterator for TopologyDecendIter<'a, W, 1> {
    type Item = Topology<'a, W, 0>;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.topology.descend(self.index);
        self.index += 1;
        next
    }
}

impl<'a, W: StructureWord> Topology<'a, W, 2> {
    /// Narrow to element `index`'s own range in the level below this one.
    /// `None` only when `index` is out of range; an empty element yields an
    /// empty child.
    pub fn descend(&self, index: usize) -> Option<Topology<'a, W, 1>> {
        let start = if *self.topology[0].get(index)? == W::RESET {
            0
        } else {
            self.topology[0].get(index)?.to_usize()
        };
        let end = self.topology[0].get(index + 1)?.to_usize();
        // An empty element has an empty child, even when the level below
        // was never written (`TopologyCache::write` omits all-empty levels).
        if start == end {
            return Some(Topology::empty());
        }
        let mut topology: [&[W]; 1] = [&[]; 1];
        topology[0] = self.topology[1].get(start..=end)?;

        Some(Topology { topology })
    }

    pub fn descend_iter(&self) -> TopologyDecendIter<'a, W, 2> {
        TopologyDecendIter {
            topology: *self,
            index: 0,
        }
    }
}

impl<'a, W: StructureWord> Iterator for TopologyDecendIter<'a, W, 2> {
    type Item = Topology<'a, W, 1>;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.topology.descend(self.index);
        self.index += 1;
        next
    }
}

impl<'a, W: StructureWord> Topology<'a, W, 3> {
    /// Narrow to element `index`'s own range in the level below this one.
    /// `None` only when `index` is out of range; an empty element yields an
    /// empty child.
    pub fn descend(&self, index: usize) -> Option<Topology<'a, W, 2>> {
        let start = if *self.topology[0].get(index)? == W::RESET {
            0
        } else {
            self.topology[0].get(index)?.to_usize()
        };
        let end = self.topology[0].get(index + 1)?.to_usize();
        // An empty element has an empty child, even when the level below
        // was never written (`TopologyCache::write` omits all-empty levels).
        if start == end {
            return Some(Topology::empty());
        }
        let mut topology: [&[W]; 2] = [&[]; 2];
        topology[0] = self.topology[1].get(start..=end)?;
        topology[1] = self.topology[2];

        Some(Topology { topology })
    }

    pub fn descend_iter(&self) -> TopologyDecendIter<'a, W, 3> {
        TopologyDecendIter {
            topology: *self,
            index: 0,
        }
    }
}

impl<'a, W: StructureWord> Iterator for TopologyDecendIter<'a, W, 3> {
    type Item = Topology<'a, W, 2>;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.topology.descend(self.index);
        self.index += 1;
        next
    }
}

impl<'a, W: StructureWord> Topology<'a, W, 4> {
    /// Narrow to element `index`'s own range in the level below this one.
    /// `None` only when `index` is out of range; an empty element yields an
    /// empty child.
    pub fn descend(&self, index: usize) -> Option<Topology<'a, W, 3>> {
        let start = if *self.topology[0].get(index)? == W::RESET {
            0
        } else {
            self.topology[0].get(index)?.to_usize()
        };
        let end = self.topology[0].get(index + 1)?.to_usize();
        // An empty element has an empty child, even when the level below
        // was never written (`TopologyCache::write` omits all-empty levels).
        if start == end {
            return Some(Topology::empty());
        }
        let mut topology: [&[W]; 3] = [&[]; 3];
        topology[0] = self.topology[1].get(start..=end)?;
        topology[1] = self.topology[2];
        topology[2] = self.topology[3];

        Some(Topology { topology })
    }

    pub fn descend_iter(&self) -> TopologyDecendIter<'a, W, 4> {
        TopologyDecendIter {
            topology: *self,
            index: 0,
        }
    }
}

impl<'a, W: StructureWord> Iterator for TopologyDecendIter<'a, W, 4> {
    type Item = Topology<'a, W, 3>;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.topology.descend(self.index);
        self.index += 1;
        next
    }
}

impl<'a, W: StructureWord> Topology<'a, W, 5> {
    /// Narrow to element `index`'s own range in the level below this one.
    /// `None` only when `index` is out of range; an empty element yields an
    /// empty child.
    pub fn descend(&self, index: usize) -> Option<Topology<'a, W, 4>> {
        let start = if *self.topology[0].get(index)? == W::RESET {
            0
        } else {
            self.topology[0].get(index)?.to_usize()
        };
        let end = self.topology[0].get(index + 1)?.to_usize();
        // An empty element has an empty child, even when the level below
        // was never written (`TopologyCache::write` omits all-empty levels).
        if start == end {
            return Some(Topology::empty());
        }
        let mut topology: [&[W]; 4] = [&[]; 4];
        topology[0] = self.topology[1].get(start..=end)?;
        topology[1] = self.topology[2];
        topology[2] = self.topology[3];
        topology[3] = self.topology[4];

        Some(Topology { topology })
    }

    pub fn descend_iter(&self) -> TopologyDecendIter<'a, W, 5> {
        TopologyDecendIter {
            topology: *self,
            index: 0,
        }
    }
}

impl<'a, W: StructureWord> Iterator for TopologyDecendIter<'a, W, 5> {
    type Item = Topology<'a, W, 4>;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.topology.descend(self.index);
        self.index += 1;
        next
    }
}

/// Whether `bounds` (a level's boundaries, sentinel excluded) never
/// decrease. Compares `W` natively and branch-free so the happy path
/// vectorizes at `W`'s own lane width; on `false`, [`level_error`] works out
/// what went wrong.
#[inline]
fn monotonic<W: StructureWord>(bounds: &[W]) -> bool {
    let mut ok = true;
    let mut prev = W::default();
    for &w in bounds {
        ok &= w >= prev;
        prev = w;
    }
    ok
}

/// [`monotonic`] for the last level of a string column, plus whether every
/// boundary falls on a char boundary of `bytes`: on a byte that isn't a
/// UTF-8 continuation byte (`0b10xx_xxxx`), or at/past the end — the caller
/// pins the last boundary to `bytes.len()`, and an ordered level never goes
/// past its last.
#[inline]
fn monotonic_on_chars<W: StructureWord>(bounds: &[W], bytes: &[u8]) -> (bool, bool) {
    let mut ordered = true;
    let mut on_chars = true;
    let mut prev = W::default();
    for &w in bounds {
        ordered &= w >= prev;
        prev = w;
        on_chars &= bytes.get(w.to_usize()).is_none_or(|&b| (b as i8) >= -0x40);
    }
    (ordered, on_chars)
}

/// The UTF-8 error for a string column [`Topology::new_str`] rejected:

/// The error for a level [`Topology::new`] rejected. `rest` starts at the
/// level's sentinel and runs to the end of the topology's words; `want` is
/// how many boundaries the level should hold (the level above's last
/// boundary — for level 0, exactly what's there). The first boundary that
/// goes backwards wins; failing that, the level's length is wrong.
#[cold]
#[inline(never)]
fn level_error<W: StructureWord>(level: usize, want: usize, rest: &[W]) -> WireError {
    let actual = rest[1..]
        .iter()
        .position(|&w| w == W::RESET)
        .unwrap_or(rest.len() - 1);
    let mut previous = 0usize;
    for (index, w) in rest.iter().enumerate().take(want.min(actual) + 1).skip(1) {
        let found = w.to_usize();
        if found < previous {
            return WireError::InvalidTopologyOrder {
                level,
                index,
                previous,
                found,
            };
        }
        previous = found;
    }
    debug_assert!(level > 0 && actual != want);
    WireError::InvalidTopologyLevel {
        level: level.saturating_sub(1),
        last: want,
        next: actual,
    }
}
