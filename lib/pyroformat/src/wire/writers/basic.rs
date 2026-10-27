//! Blobs: strings and byte buffers.
//!
//! A blob is an opaque run of bytes, the same shape on the wire as a
//! `Vec<u8>`, its extent the byte length rather than the character count. So
//! each is just a [`Run`] of `u8`, and the `u8` leaf and the blanket impls in
//! `nesting.rs` do the rest: a bare blob is one `(start, byte count)` slot,
//! and every list of blobs, at any depth, keeps its tiers above that.

use std::{borrow::Cow, rc::Rc, sync::Arc};

use crate::wire::writers::run::Run;

impl Run for str {
    type Item = u8;

    #[inline]
    fn as_run(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl Run for String {
    type Item = u8;

    #[inline]
    fn as_run(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl Run for Box<str> {
    type Item = u8;

    #[inline]
    fn as_run(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl Run for Arc<str> {
    type Item = u8;

    #[inline]
    fn as_run(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl Run for Rc<str> {
    type Item = u8;

    #[inline]
    fn as_run(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl Run for Cow<'_, str> {
    type Item = u8;

    #[inline]
    fn as_run(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl Run for bytes::Bytes {
    type Item = u8;

    #[inline]
    fn as_run(&self) -> &[u8] {
        self
    }
}

impl Run for bytes::BytesMut {
    type Item = u8;

    #[inline]
    fn as_run(&self) -> &[u8] {
        self
    }
}
