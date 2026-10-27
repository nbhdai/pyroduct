//! `Option<BitVec<u8, Lsb0>>` is out of scope for v1 (an empty `BitVec`
//! already represents "no bits") and should fail cleanly.

use bitvec::order::Lsb0;
use bitvec::vec::BitVec;
use pyroformat::{Layout, Writer, Reader};

#[derive(Layout, Writer, Reader)]
struct WithOptionBitVec {
    flags: Option<BitVec<u8, Lsb0>>,
}

fn main() {}
