//! `BitVec<u16, Lsb0>` (wrong backing storage) is not supported — only
//! `<u8, Lsb0>` matches the wire's byte layout — and should fail cleanly.

use bitvec::order::Lsb0;
use bitvec::vec::BitVec;
use pyroformat::{Layout, Writer, Reader};

#[derive(Layout, Writer, Reader)]
struct WithBadStorage {
    flags: BitVec<u16, Lsb0>,
}

fn main() {}
