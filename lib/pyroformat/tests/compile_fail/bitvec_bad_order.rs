//! `BitVec<u8, Msb0>` (wrong bit order) is not supported — only `<u8, Lsb0>`
//! matches the wire's byte layout — and should fail cleanly.

use bitvec::order::Msb0;
use bitvec::vec::BitVec;
use pyroformat::{Layout, Writer, Reader};

#[derive(Layout, Writer, Reader)]
struct WithBadOrder {
    flags: BitVec<u8, Msb0>,
}

fn main() {}
