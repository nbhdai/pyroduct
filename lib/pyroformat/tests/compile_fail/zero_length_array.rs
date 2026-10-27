//! A fixed array needs at least one element: an inode's `fixed_size: 0`
//! means "not a fixed array". Required, optional, and under a list.

use pyroformat::{Layout, Reader, Writer};

#[derive(Layout, Writer, Reader)]
#[pyro(Word = u32)]
struct Required {
    a: [u8; 0],
}

#[derive(Layout, Writer, Reader)]
#[pyro(Word = u32)]
struct Optional {
    a: Option<[u8; 0]>,
}

#[derive(Layout, Writer, Reader)]
#[pyro(Word = u32)]
struct InList {
    a: Vec<Option<[u16; 0]>>,
}

fn main() {}
