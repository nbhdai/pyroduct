use std::marker::PhantomData;

use bitvec::{domain::Domain, slice::BitSlice};

use crate::{
    Layout, MAX_TIERS,
    buf::PyroSink,
    errors::WireError,
    schema::{Inode, StructureWord, WireKind},
    wire::writers::InodeOffset,
};

pub struct PreBuffer<'a, Inodes: Layout, const FIXED: usize, const INODE_LEN: usize> {
    pub fixed: &'a mut [u8; FIXED],
    pub variable: &'a mut [u8],
    pub sizes: &'a mut [InodeSize<Inodes::Word>; INODE_LEN],
    phantom: PhantomData<Inodes>,
}

#[derive(Default, Clone)]
pub struct ValidityWriter<W: StructureWord> {
    /// **Pre fixed write:**
    /// Count of the number of valid samples we have to write.
    /// **Post fixed write:**
    /// Where we have to start writing the bit vector from.
    pub start: W,
    pub byte: u8,
    pub bit: u8,
}

#[derive(Default, Clone)]
pub struct InodeSize<W: StructureWord> {
    /// **Pre fixed write:**
    /// Count of the number of elements in each topology level
    /// **Post fixed write:**
    /// Where we have to start writing from for each topology level
    pub topo: [W; MAX_TIERS],
    /// **Pre fixed write:**
    /// Count of the number of samples we have to write.
    /// **Post fixed write:**
    /// Where we have to start data writing from
    pub data: W,
    /// Bit-packed leaves (bools): the next bit within `data`'s byte.
    pub data_bit: u8,
    pub validity: ValidityWriter<W>,
}

/// Checks every repeated bool and `Option` leaf's planned sample count fits
/// the word. A count stops at `RESET` rather than wrapping (see
/// [`StructureWord::add_planned`]), so one that reached it is too big. These
/// leaves can plan fewer bytes than samples, so the plan's total alone
/// doesn't catch them.
///
/// `checked` is the layout's [`checked_sizes`], a const: only those inodes
/// are looked at, so a layout with none checks nothing.
#[inline(always)]
pub fn check_sizes<W: StructureWord, const INODE_LEN: usize, const K: usize>(
    checked: &[usize; K],
    sizes: &[InodeSize<W>; INODE_LEN],
) -> Result<(), WireError> {
    let max = W::RESET.to_usize();
    for &i in checked {
        let count = sizes[i].data.to_usize();
        if count >= max {
            return Err(WireError::OffsetOverflow { end: count, max });
        }
    }
    Ok(())
}

/// Whether [`check_sizes`] looks at `inode`: a bool or `Option` leaf that
/// repeats. One that doesn't (`nesting == 0`, not `group_nest`) is a single
/// fixed slot or nullable bit, and its count never grows.
const fn size_checked_inode(inode: &Inode) -> bool {
    size_checked(inode.kind) && (inode.nesting() > 0 || inode.group_nest())
}

/// How many of `inodes` [`check_sizes`] looks at: the length of
/// [`checked_sizes`].
pub const fn checked_sizes_len(inodes: &[Inode]) -> usize {
    let mut count = 0;
    let mut i = 0;
    while i < inodes.len() {
        if size_checked_inode(&inodes[i]) {
            count += 1;
        }
        i += 1;
    }
    count
}

/// The indices of the inodes [`check_sizes`] looks at, `K` of them (from
/// [`checked_sizes_len`]).
pub const fn checked_sizes<const K: usize>(inodes: &[Inode]) -> [usize; K] {
    let mut checked = [0; K];
    let mut k = 0;
    let mut i = 0;
    while i < inodes.len() {
        if size_checked_inode(&inodes[i]) {
            checked[k] = i;
            k += 1;
        }
        i += 1;
    }
    assert!(k == K);
    checked
}

/// A leaf whose planned count is at most its planned bytes: a non-bool,
/// non-`Option` primitive (or a fixed array of them), or a string.
const fn count_in_bytes(kind: WireKind) -> bool {
    matches!(
        kind,
        WireKind::U8
            | WireKind::U16
            | WireKind::U32
            | WireKind::U64
            | WireKind::I8
            | WireKind::I16
            | WireKind::I32
            | WireKind::I64
            | WireKind::F16
            | WireKind::F32
            | WireKind::F64
            | WireKind::Timestamp
            | WireKind::Str
            | WireKind::Bytes
    )
}

/// A bool or `Option` leaf: one that can plan fewer bytes than samples.
const fn size_checked(kind: WireKind) -> bool {
    matches!(
        kind,
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
            | WireKind::OptionGroup
    )
}

impl<'a, Inodes: Layout, const FIXED: usize, const INODE_LEN: usize>
    PreBuffer<'a, Inodes, FIXED, INODE_LEN>
{
    /// Plans `n` more of inode `O::N`'s own count (`data`).
    ///
    /// A count that is a byte length, or bounded by one (a non-bool leaf's
    /// elements, a string's bytes), is covered by the plan's byte total,
    /// which the writer checks against `RESET` once: it adds plainly. Every
    /// other count (bools and `Option` leaves, which can plan fewer bytes
    /// than samples; groups; maps) stops at `RESET` instead, for
    /// [`check_sizes`] to see. Which one is decided at compile time.
    #[inline(always)]
    pub fn plan_data<O: InodeOffset>(&mut self, n: usize) {
        if const { count_in_bytes(Inodes::INODES[O::N].kind) } {
            self.sizes[O::N].data.add_usize(n);
        } else {
            self.sizes[O::N].data.add_planned(n);
        }
    }

    /// Splits the sink's payload after its first `FIXED` bytes, counting into
    /// `sizes`: zeroed for a plan, the plan's counts for the passes after it.
    /// The counts live outside the buffer so the plan's carry over to the
    /// passes, after the sink has grown, without a copy.
    ///
    /// # Panics
    /// If the payload is shorter than `FIXED`.
    #[inline]
    pub fn new<S: PyroSink>(
        sink: &'a mut S,
        sizes: &'a mut [InodeSize<Inodes::Word>; INODE_LEN],
    ) -> Self {
        Self::from_payload(sink.payload_mut(), sizes)
    }

    /// [`Self::new`] over a bare `payload`.
    ///
    /// # Panics
    /// If `payload` is shorter than `FIXED`.
    #[inline]
    pub fn from_payload(
        payload: &'a mut [u8],
        sizes: &'a mut [InodeSize<Inodes::Word>; INODE_LEN],
    ) -> Self {
        let (fixed, variable) = payload.split_at_mut(FIXED);
        Self {
            fixed: fixed.try_into().unwrap(),
            variable,
            sizes,
            phantom: PhantomData,
        }
    }

    /// The variable region from payload offset `at` on.
    #[inline(always)]
    pub fn variable_at(&mut self, at: usize) -> &mut [u8] {
        &mut self.variable[at - FIXED..]
    }

    #[inline(always)]
    pub fn pad_to(&mut self, byte_offset: usize, align: usize) -> usize {
        let aligned = byte_offset.next_multiple_of(align);
        self.variable[byte_offset - FIXED..aligned - FIXED].fill(0);
        aligned
    }

    #[inline]
    pub fn set_validity_bit<O: InodeOffset>(&mut self, present: bool) {
        let bit_index = Inodes::INODES[O::N].nullable_bit_index() as usize;

        let mask = 1u8 << (bit_index % 8);
        let byte = &mut self.fixed[Inodes::VALIDITY_START + bit_index / 8];
        if present {
            *byte |= mask;
        } else {
            *byte &= !mask;
        }
    }

    #[inline]
    pub fn push_validity_bit<O: InodeOffset>(&mut self, present: bool) {
        if present {
            self.sizes[O::N].validity.byte |= 1 << self.sizes[O::N].validity.bit;
        }
        self.sizes[O::N].validity.bit += 1;
        let start = self.sizes[O::N].validity.start.to_usize() - FIXED;
        // Written on every push, not only once the byte fills: the pre
        // writers have no end-of-run hook to call `finish` from, so the
        // trailing partial byte has to already be in place.
        self.variable[start] = self.sizes[O::N].validity.byte;
        if self.sizes[O::N].validity.bit == 8 {
            self.sizes[O::N].validity.byte = 0;
            self.sizes[O::N].validity.bit = 0;
            self.sizes[O::N].validity.start.add_usize(1);
        }
    }

    /// Pushes every bit of `bits`, in order.
    ///
    /// While this writer and `bits` both sit on a byte boundary, the whole
    /// bytes go in one copy; only what is left over goes bit by bit. Since
    /// the copy stops at the last whole byte, whatever a `BitVec` keeps in
    /// its last byte past its length never reaches the wire.
    #[inline]
    pub fn extend_validity<O: InodeOffset>(&mut self, bits: &BitSlice<u8>) {
        let mut rest = bits;
        if self.sizes[O::N].validity.bit == 0
            && let Domain::Region {
                head: None, body, ..
            } = bits.domain()
        {
            let start = self.sizes[O::N].validity.start.to_usize() - FIXED;

            self.variable[start..start + body.len()].copy_from_slice(body);
            self.sizes[O::N].validity.start.add_usize(body.len());
            rest = &bits[body.len() * 8..];
        }
        self.extend_validity_iter::<O>(rest.iter().by_vals());
    }

    /// Pushes every bit of `bits`, in order.
    ///
    /// Up to the next byte boundary it goes as [`Self::extend_validity_iter`]; from
    /// there each whole byte is packed from eight bools in a register and
    /// stored once, and the leftover goes as [`Self::extend_validity_iter`] again.
    #[inline]
    pub fn extend_validity_slice<O: InodeOffset>(&mut self, bits: &[bool]) {
        let head = ((8 - self.sizes[O::N].validity.bit as usize) % 8).min(bits.len());
        let (head, rest) = bits.split_at(head);
        self.extend_validity_iter::<O>(head.iter().copied());

        // Past the head this writer is on a byte boundary, or `rest` is empty.
        let chunks = rest.chunks_exact(8);
        let tail = chunks.remainder();
        let start = self.sizes[O::N].validity.start.to_usize() - FIXED;
        let len = chunks.len();
        for (out, chunk) in self.variable[start..start + len].iter_mut().zip(chunks) {
            *out = chunk
                .iter()
                .enumerate()
                .fold(0, |byte, (i, &bit)| byte | ((bit as u8) << i));
        }
        self.sizes[O::N].validity.start.add_usize(len);
        self.extend_validity_iter::<O>(tail.iter().copied());
    }

    /// Pushes every bit `bits` yields, in order: a bulk [`Self::push_validity_bit`] for
    /// a whole run of `Option`s' presence at once.
    ///
    /// The byte being filled is kept in a register and stored once it is
    /// whole, not on every bit; only the trailing partial byte is stored at
    /// the end, so it is in place as [`Self::push_validity_bit`] leaves it.
    #[inline]
    pub fn extend_validity_iter<O: InodeOffset>(&mut self, bits: impl IntoIterator<Item = bool>) {
        let mut start = self.sizes[O::N].validity.start.to_usize() - FIXED;
        let mut byte = self.sizes[O::N].validity.byte;
        let mut bit = self.sizes[O::N].validity.bit;
        for present in bits {
            byte |= (present as u8) << bit;
            bit += 1;
            if bit == 8 {
                self.variable[start] = byte;
                start += 1;
                byte = 0;
                bit = 0;
            }
        }
        if bit > 0 {
            self.variable[start] = byte;
        }
        self.sizes[O::N].validity.start = Inodes::Word::from_usize(start + FIXED);
        self.sizes[O::N].validity.byte = byte;
        self.sizes[O::N].validity.bit = bit;
    }

    /// Flushes the partial trailing byte, then pads the bitmap out to
    /// `alignment` so the next region starts where the layout expects.
    #[inline]
    pub fn finish_validity<O: InodeOffset>(self) -> Result<(), WireError> {
        if self.sizes[O::N].validity.bit > 0 {
            let start = self.sizes[O::N].validity.start.to_usize() - FIXED;
            self.variable[start] = self.sizes[O::N].validity.byte;
        }
        Ok(())
    }

    pub(crate) fn as_le_bytes<T: Copy>(slice: &[T]) -> &[u8] {
        // SAFETY: `T` is `Copy` (no Drop glue to worry about) and, per the
        // doc comment above, one of the wire format's plain fixed-width numeric
        // types with no padding bytes. Reading its bytes can never be invalid,
        // and on the little-endian-only targets this crate compiles for, those
        // bytes are exactly the wire encoding `to_le_bytes()` would produce.
        unsafe {
            std::slice::from_raw_parts(slice.as_ptr() as *const u8, std::mem::size_of_val(slice))
        }
    }
}
