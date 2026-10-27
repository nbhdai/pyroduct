// =============================================================================
// PyroSchema<'a>
// =============================================================================

use std::borrow::Cow;

use crate::{
    types::{
        ChildField, ChildIter, FieldName, FieldNames, PyroField, PyroType, field::field_name_to_cow,
    },
    wire::InodeMeta,
    word::Word,
};

/// An ordered collection of top-level fields.
///
/// Names are stored contiguously in a single `FieldNames` with a
/// parallel `Vec<PyroType>` for types.
///
/// ```text
/// names: ["id", "name", "scores", "address", "street", "city"]
/// types: [I64,  Str,    PrimitiveList(F32),  Group([Str, Str])]
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PyroSchema<'a> {
    doc: FieldName<'a>,
    names: FieldNames<'a>,
    field_docs: FieldNames<'a>,
    types: Vec<PyroType>,
    meta: InodeMeta,
}

impl<'a> PyroSchema<'a> {
    /// Build from pre-built fields. Flattens names into contiguous storage.
    pub fn new(fields: Vec<PyroField<'a>>) -> Self {
        let total: usize = fields.iter().map(|f| 1 + f.child_names.len()).sum();
        let mut names: Vec<Cow<'a, str>> = Vec::with_capacity(total);
        let mut docs: Vec<Cow<'a, str>> = Vec::with_capacity(total);
        let mut types = Vec::with_capacity(fields.len());

        for field in fields {
            names.push(field_name_to_cow(field.name));
            docs.push(field_name_to_cow(field.doc));
            match field.child_names {
                FieldNames::Owned(v) => names.extend(v),
                FieldNames::Borrowed(s) => names.extend_from_slice(s),
                FieldNames::Static(s) => {
                    names.extend(s.iter().map(|&r| Cow::Borrowed(r)));
                }
                FieldNames::Empty(l) => names.extend(vec!["".into(); l]),
            }
            match field.child_docs {
                FieldNames::Owned(v) => docs.extend(v),
                FieldNames::Borrowed(s) => docs.extend_from_slice(s),
                FieldNames::Static(s) => {
                    docs.extend(s.iter().map(|&r| Cow::Borrowed(r)));
                }
                FieldNames::Empty(l) => docs.extend(vec!["".into(); l]),
            }
            types.push(field.datatype);
        }

        Self {
            doc: FieldName::Static(""),
            names: FieldNames::Owned(names),
            field_docs: FieldNames::Owned(docs),
            types,
            meta: InodeMeta::DEFAULT,
        }
    }

    pub fn set_word(&mut self, word: Word) {
        self.meta.word = word;
    }

    pub fn word(&self) -> Word {
        self.meta.word
    }

    pub fn meta(&self) -> InodeMeta {
        self.meta
    }

    pub fn with_doc(mut self, doc: impl Into<FieldName<'a>>) -> Self {
        self.doc = doc.into();
        self
    }

    pub fn with_field_docs(mut self, field_docs: FieldNames<'a>) -> Self {
        debug_assert_eq!(self.names.len(), field_docs.len());
        self.field_docs = field_docs;
        self
    }

    /// Build directly from raw parts.
    pub fn from_raw(names: FieldNames<'a>, types: Vec<PyroType>, meta: InodeMeta) -> Self {
        debug_assert_eq!(
            names.len(),
            types.iter().map(|t| 1 + t.name_count()).sum::<usize>(),
        );
        Self {
            doc: FieldName::Static(""),
            field_docs: FieldNames::empty(names.len()),
            names,
            types,
            meta,
        }
    }

    pub fn empty() -> Self {
        Self {
            doc: FieldName::Static(""),
            names: FieldNames::empty(0),
            field_docs: FieldNames::empty(0),
            types: vec![],
            meta: InodeMeta::DEFAULT,
        }
    }

    #[inline]
    pub fn doc(&self) -> &str {
        self.doc.as_ref()
    }

    #[inline]
    pub fn names(&self) -> &FieldNames<'a> {
        &self.names
    }

    #[inline]
    pub fn field_docs(&self) -> &FieldNames<'a> {
        &self.field_docs
    }

    #[inline]
    pub fn types(&self) -> &[PyroType] {
        &self.types
    }

    #[inline]
    pub fn num_fields(&self) -> usize {
        self.types.len()
    }

    /// Get the i-th top-level field as a `ChildField`.
    pub fn field(&self, index: usize) -> ChildField<'_> {
        assert!(index < self.types.len());
        let mut offset = 0;
        for i in 0..index {
            offset += 1 + self.types[i].name_count();
        }
        let nc = self.types[index].name_count();
        ChildField {
            name: self.names.get(offset),
            doc: self.field_docs.get(offset),
            child_names: &self.names,
            child_docs: &self.field_docs,
            child_names_offset: offset + 1,
            child_names_len: nc,
            datatype: &self.types[index],
        }
    }

    /// Iterate all top-level fields.
    pub fn fields(&self) -> ChildIter<'_> {
        ChildIter {
            parent_names: &self.names,
            parent_docs: &self.field_docs,
            child_types: &self.types,
            offset: 0,
            index: 0,
        }
    }

    pub fn field_with_name(&self, name: &str) -> Option<ChildField<'_>> {
        self.fields().find(|f| f.name() == name)
    }

    pub fn index_of(&self, name: &str) -> Option<usize> {
        self.fields().position(|f| f.name() == name)
    }
}
