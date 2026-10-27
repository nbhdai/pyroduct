//! `pyro_optimize!` over a multi-struct message: the whole point of the
//! macro, and the case a per-struct `#[derive(Layout, Writer, Reader)]` cannot serve.
//!
//! A lone derive sees one struct's syntax and nothing else, so a
//! struct-shaped field can never resolve to a sibling — which makes any
//! nesting struct ineligible for the const-offset path. Putting the structs
//! in one `pyro_optimize!` block puts them in one collection, and then the
//! macro can lay a nested substruct out inline and give every leaf a literal
//! offset.
//!
//! The block is meant to read as ordinary Rust: any struct tagged
//! `#[derive(Layout, Writer, Reader)]` — with or without other derives alongside it — is
//! picked up, declaration order does not matter, and doc comments, field
//! attributes and visibility all survive.
//!
//! `pyro_schema_header()` is the observable marker for "this struct took the
//! optimized path": it is a `.rodata` blob assembled at macro-expansion time
//! from the fully flattened name/doc table, which is exactly what a struct
//! reaching outside its collection cannot have. Calling it below is
//! therefore a compile-time assertion that these structs were optimized, not
//! merely that they compiled.

use pyroformat::{Reader, Layout, Writer};

// Declared before the struct it nests, and before `Point` exists —
// the collection is topologically ordered internally, so declaration
// order is the author's to choose.
/// A line segment, with a label.
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
pub struct Segment32 {
    /// What this segment is called.
    pub name: String,
    pub start: Point32,
    pub end: Point32,
}

/// A line segment, with a label.
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
pub struct Segment16 {
    /// What this segment is called.
    pub name: String,
    pub start: Point16,
    pub end: Point16,
}

/// A point in the plane.
#[derive(Debug, Layout, Writer, Reader, PartialEq, Clone)]
#[pyro(Word = u32)]
pub struct Point32 {
    /// The horizontal coordinate.
    pub x: i32,
    pub y: i32,
}

/// A point in the plane.
#[derive(Debug, Layout, Writer, Reader, PartialEq, Clone)]
#[pyro(Word = u16)]
pub struct Point16 {
    /// The horizontal coordinate.
    pub x: i32,
    pub y: i32,
}

// Three levels deep, to exercise the recursive flatten rather than just
// one hop.
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
pub struct Drawing32 {
    pub id: u64,
    pub main: Segment32,
    pub scale: Option<u32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
pub struct Drawing16 {
    pub id: u64,
    pub main: Segment16,
    pub scale: Option<u32>,
}

// Only the marker derive is required; everything else is optional.
#[derive(Layout, Writer, Reader)]
#[pyro(Word = u32)]
struct Bare32 {
    v: u64,
}

#[derive(Layout, Writer, Reader)]
#[pyro(Word = u16)]
struct Bare16 {
    v: u64,
}

// `#[pyro(..)]` field attributes work inside a block. They are the
// reason the block parser *swaps* the marker derive for `PyroBlockAttrs`
// rather than deleting it: the marker's own `attributes(pyro)` is what
// tells rustc `pyro` is a recognized inert attribute, and with the
// marker gone from the re-emitted item something still has to say so.
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
pub struct WithAttrs32 {
    /// Read as a numeric list rather than an opaque byte blob.
    #[pyro(numeric)]
    pub samples: Vec<u8>,
    #[pyro(allow(vec_bool))]
    pub flags: Vec<bool>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
pub struct WithAttrs16 {
    /// Read as a numeric list rather than an opaque byte blob.
    #[pyro(numeric)]
    pub samples: Vec<u8>,
    #[pyro(allow(vec_bool))]
    pub flags: Vec<bool>,
}

fn assert_pyro_traits<T: Layout + Writer + Reader>() {}

fn main() {}
