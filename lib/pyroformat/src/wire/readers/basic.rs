use crate::{
    Layout,
    errors::WireError,
    offsets::InodeOffset,
    refs::{Nested, StrNested},
    wire::readers::{ForeignReader, SplitSource},
};
use pyro_schema::StructureWord;

/// Every wire shape an opaque run of bytes can take, for one such type —
/// the read side of `writers/basic.rs`.
///
/// A blob is already a tier of its own (its extent is a byte count), so a
/// bare blob reads with no topology, its iterated form with one level, and
/// each `Vec` above it adds one more, up to six.
///
/// `View<'a, W, N>` is the borrowed view at `N` topology levels and `view`
/// loads one (topology and all) from the buffer; `$leaf` (naming the impl's
/// own `'a` and `W`) and `leaf` are what a bare blob reads back as.
macro_rules! bytes_reader {
    (@shape $self:ty, $ref_n:literal, $nest_n:literal) => {
        impl ForeignReader for $self {

            type Ref<'a, W: StructureWord> = View<'a, W, $ref_n>;

            type NestedRef<'a, W: StructureWord> = View<'a, W, $nest_n>;

            fn foreign_read<
                Inodes: Layout, O: InodeOffset, const FIXED: usize
            >(
                buffer: SplitSource<'_, Inodes, FIXED>,
            ) -> Result<Self::Ref<'_, Inodes::Word>, WireError> {
                view::<Inodes, O, FIXED, $ref_n>(buffer)
            }

            fn foreign_iter_read<
                Inodes: Layout, O: InodeOffset, const FIXED: usize
            >(
                buffer: SplitSource<'_, Inodes, FIXED>,
            ) -> Result<Self::NestedRef<'_, Inodes::Word>, WireError> {
                view::<Inodes, O, FIXED, $nest_n>(buffer)
            }
        }
    };

    ($ty:ty, $leaf:ty) => {
        impl ForeignReader for $ty {

            type Ref<'a, W: StructureWord> = $leaf;

            type NestedRef<'a, W: StructureWord> = View<'a, W, 1>;

            fn foreign_read<
                Inodes: Layout, O: InodeOffset, const FIXED: usize
            >(
                buffer: SplitSource<'_, Inodes, FIXED>,
            ) -> Result<Self::Ref<'_, Inodes::Word>, WireError> {
                leaf::<Inodes, O, FIXED>(buffer)
            }

            fn foreign_iter_read<
                Inodes: Layout, O: InodeOffset, const FIXED: usize
            >(
                buffer: SplitSource<'_, Inodes, FIXED>,
            ) -> Result<Self::NestedRef<'_, Inodes::Word>, WireError> {
                view::<Inodes, O, FIXED, 1>(buffer)
            }
        }

        bytes_reader!(@shape [$ty], 1, 2);
        bytes_reader!(@shape Vec<$ty>, 1, 2);
        bytes_reader!(@shape [Vec<$ty>], 2, 3);
        bytes_reader!(@shape Vec<Vec<$ty>>, 2, 3);
        bytes_reader!(@shape [Vec<Vec<$ty>>], 3, 4);
        bytes_reader!(@shape Vec<Vec<Vec<$ty>>>, 3, 4);
        bytes_reader!(@shape [Vec<Vec<Vec<$ty>>>], 4, 5);
        bytes_reader!(@shape Vec<Vec<Vec<Vec<$ty>>>>, 4, 5);
        bytes_reader!(@shape [Vec<Vec<Vec<Vec<$ty>>>>], 5, 6);
        bytes_reader!(@shape Vec<Vec<Vec<Vec<Vec<$ty>>>>>, 5, 6);
    };
}

/// Byte containers read back like `Vec<u8>`: a bare blob is `&[u8]`, and
/// each tier above it is a `Nested<W, u8, N>` level.
mod bytes_views {
    use super::*;

    type View<'a, W, const N: usize> = Nested<'a, W, u8, N>;

    fn leaf<'a, Inodes: Layout, O: InodeOffset, const FIXED: usize>(
        buffer: SplitSource<'a, Inodes, FIXED>,
    ) -> Result<&'a [u8], WireError> {
        buffer.read_u8_slice::<O>()
    }

    fn view<'a, Inodes: Layout, O: InodeOffset, const FIXED: usize, const N: usize>(
        buffer: SplitSource<'a, Inodes, FIXED>,
    ) -> Result<View<'a, Inodes::Word, N>, WireError> {
        buffer.read_u8_nest::<O, N>()
    }

    bytes_reader!(Box<[u8]>, &'a [u8]);
    bytes_reader!(bytes::Bytes, &'a [u8]);
    bytes_reader!(bytes::BytesMut, &'a [u8]);
}

/// Strings load through [`SplitSource::read_str_nest`], which checks the
/// topology against the bytes and every string as UTF-8 in one pass.
mod str_views {
    use super::*;

    type View<'a, W, const N: usize> = StrNested<'a, W, N>;

    fn leaf<'a, Inodes: Layout, O: InodeOffset, const FIXED: usize>(
        buffer: SplitSource<'a, Inodes, FIXED>,
    ) -> Result<&'a str, WireError> {
        buffer.read_str::<O>()
    }

    fn view<'a, Inodes: Layout, O: InodeOffset, const FIXED: usize, const N: usize>(
        buffer: SplitSource<'a, Inodes, FIXED>,
    ) -> Result<View<'a, Inodes::Word, N>, WireError> {
        buffer.read_str_nest::<O, N>()
    }

    bytes_reader!(String, &'a str);
}
