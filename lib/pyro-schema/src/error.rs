use crate::wire::InodeDecodeError;

/// Errors produced when decoding wire-format type information.
#[derive(Debug, thiserror::Error)]
pub enum WireKindError {
    #[error("invalid wire kind discriminant: {0}")]
    InvalidKind(u8),
    #[error("invalid inner wire kind: {0}")]
    InvalidInner(u8),
}

/// Errors from the schema layer: decoding an [`Inode`](crate::Inode) table
/// or a metadata (names/docs) string table.
///
/// Deliberately minimal and free of anything transport- or buffer-related
/// (`pyroformat::errors::WireError` carries those) — this crate describes
/// what a schema *is*, not how it moved.
#[derive(Debug, thiserror::Error)]
pub enum SchemaError {
    #[error("invalid wire kind: {0}")]
    InvalidKind(#[from] WireKindError),
    #[error(transparent)]
    InodeDecode(#[from] InodeDecodeError),
    #[error("invalid utf-8 in string payload")]
    InvalidUtf8,
    #[error("field id {0} out of range")]
    FieldOutOfRange(u16),
}
