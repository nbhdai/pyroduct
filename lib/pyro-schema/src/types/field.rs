// =============================================================================
// PyroField<'a>
// =============================================================================

use std::borrow::Cow;

use crate::types::{FieldName, FieldNames, PyroType};

/// A named, typed column descriptor.
///
/// `name` is this field's own name.
///
/// `child_names` is a flat, pre-order packing of names for every `Group`
/// reachable from `datatype` — including through `List` and `Map`.
/// For leaf types, `child_names` is empty.
///
/// For `Group([Str, Group([Str, Str]), F64])`:
/// ```text
/// child_names: ["name", "address", "street", "city", "score"]
/// ```
///
/// For `List(Group([Str, I32]), true)`:
/// ```text
/// child_names: ["x", "y"]
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PyroField<'a> {
    pub(crate) name: FieldName<'a>,
    pub(crate) doc: FieldName<'a>,
    pub(crate) child_names: FieldNames<'a>,
    pub(crate) child_docs: FieldNames<'a>,
    pub(crate) datatype: PyroType,
}

impl<'a> PyroField<'a> {
    /// Create a leaf field (type has no nested Groups).
    pub fn new(name: impl Into<FieldName<'a>>, datatype: PyroType) -> Self {
        debug_assert_eq!(
            datatype.name_count(),
            0,
            "Type requires child names — use with_children() or group()"
        );
        Self {
            name: name.into(),
            doc: FieldName::Static(""),
            child_names: FieldNames::empty(0),
            child_docs: FieldNames::empty(0),
            datatype,
        }
    }

    pub fn with_doc(mut self, doc: impl Into<FieldName<'a>>) -> Self {
        self.doc = doc.into();
        self
    }

    /// Create a field with explicit child names for types containing Groups.
    pub fn with_children(
        name: impl Into<FieldName<'a>>,
        child_names: FieldNames<'a>,
        datatype: PyroType,
    ) -> Self {
        debug_assert_eq!(
            datatype.name_count(),
            child_names.len(),
            "child_names.len() must equal type.name_count()"
        );
        let n = child_names.len();
        Self {
            name: name.into(),
            doc: FieldName::Static(""),
            child_names,
            child_docs: FieldNames::empty(n),
            datatype,
        }
    }

    pub fn with_children_and_docs(
        name: impl Into<FieldName<'a>>,
        child_names: FieldNames<'a>,
        child_docs: FieldNames<'a>,
        datatype: PyroType,
    ) -> Self {
        debug_assert_eq!(
            datatype.name_count(),
            child_names.len(),
            "child_names.len() must equal type.name_count()"
        );
        debug_assert_eq!(
            child_names.len(),
            child_docs.len(),
            "child_names.len() must equal child_docs.len()"
        );
        Self {
            name: name.into(),
            doc: FieldName::Static(""),
            child_names,
            child_docs,
            datatype,
        }
    }

    /// Build a Group field from child `PyroField`s.
    /// Flattens children's names into a single contiguous `child_names`.
    pub fn group(name: impl Into<FieldName<'a>>, children: Vec<PyroField<'a>>) -> Self {
        let total: usize = children.iter().map(|c| 1 + c.child_names.len()).sum();
        let mut names: Vec<Cow<'a, str>> = Vec::with_capacity(total);
        let mut docs: Vec<Cow<'a, str>> = Vec::with_capacity(total);
        let mut child_types = Vec::with_capacity(children.len());

        for child in children {
            names.push(field_name_to_cow(child.name));
            docs.push(field_name_to_cow(child.doc));

            match child.child_names {
                FieldNames::Owned(v) => names.extend(v),
                FieldNames::Borrowed(s) => names.extend_from_slice(s),
                FieldNames::Static(s) => {
                    names.extend(s.iter().map(|&r| Cow::Borrowed(r)));
                }
                FieldNames::Empty(l) => names.extend(vec!["".into(); l]),
            }
            match child.child_docs {
                FieldNames::Owned(v) => docs.extend(v),
                FieldNames::Borrowed(s) => docs.extend_from_slice(s),
                FieldNames::Static(s) => {
                    docs.extend(s.iter().map(|&r| Cow::Borrowed(r)));
                }
                FieldNames::Empty(l) => docs.extend(vec!["".into(); l]),
            }
            child_types.push(child.datatype);
        }

        Self {
            name: name.into(),
            child_names: FieldNames::Owned(names),
            doc: FieldName::Static(""),
            datatype: PyroType::Group(child_types.into()),
            child_docs: FieldNames::Owned(docs),
        }
    }

    #[inline]
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    #[inline]
    pub fn doc(&self) -> &str {
        self.doc.as_ref()
    }

    #[inline]
    pub fn datatype(&self) -> &PyroType {
        &self.datatype
    }

    #[inline]
    pub fn child_names(&self) -> &FieldNames<'a> {
        &self.child_names
    }

    #[inline]
    pub fn child_docs(&self) -> &FieldNames<'a> {
        &self.child_docs
    }

    pub fn num_children(&self) -> usize {
        match &self.datatype {
            PyroType::Group(ct) | PyroType::OptionGroup(ct) => ct.len(),
            _ => 0,
        }
    }

    /// Get the i-th direct child of a Group as an owned `PyroField`
    /// that borrows names from `self`.
    pub fn group_child(&self, index: usize) -> ChildField<'_> {
        let child_types = match &self.datatype {
            PyroType::Group(ct) | PyroType::OptionGroup(ct) => ct,
            _ => panic!("group_child() on non-Group: {:?}", self.datatype),
        };
        assert!(index < child_types.len());

        let mut offset = 0;
        for i in 0..index {
            offset += 1 + child_types[i].name_count();
        }
        let nc = child_types[index].name_count();

        ChildField {
            name: self.child_names.get(offset),
            doc: self.child_docs.get(offset),
            child_names: &self.child_names,
            child_docs: &self.child_docs,
            child_names_offset: offset + 1,
            child_names_len: nc,
            datatype: &child_types[index],
        }
    }

    /// Iterate children of a Group.
    pub fn group_children(&self) -> Option<ChildIter<'_>> {
        match &self.datatype {
            PyroType::Group(child_types) | PyroType::OptionGroup(child_types) => Some(ChildIter {
                parent_names: &self.child_names,
                parent_docs: &self.child_docs,
                child_types,
                offset: 0,
                index: 0,
            }),
            _ => None,
        }
    }

    /// If this is one or more `List` layers wrapping a `Group`/`OptionGroup`,
    /// iterate the inner group's children.
    pub fn list_group_children(&self) -> Option<ChildIter<'_>> {
        let (nesting, _, child_types) = self.datatype.list_group_nesting()?;
        if nesting == 0 {
            return None;
        }
        Some(ChildIter {
            parent_names: &self.child_names,
            parent_docs: &self.child_docs,
            child_types,
            offset: 0,
            index: 0,
        })
    }

    /// If this is `Map { key: Group(...), .. }`, iterate the key group's children.
    pub fn map_key_group_children(&self) -> Option<ChildIter<'_>> {
        match &self.datatype {
            PyroType::Map { key, .. } => {
                key.list_group_children_types()
                    .map(|child_types| ChildIter {
                        parent_names: &self.child_names,
                        parent_docs: &self.child_docs,
                        child_types,
                        offset: 0,
                        index: 0,
                    })
            }
            _ => None,
        }
    }

    /// If this is `Map { value: Group(...), .. }`, iterate the value group's children.
    pub fn map_value_group_children(&self) -> Option<ChildIter<'_>> {
        match &self.datatype {
            PyroType::Map { key, value } => {
                let offset = key.name_count();
                value
                    .list_group_children_types()
                    .map(|child_types| ChildIter {
                        parent_names: &self.child_names,
                        parent_docs: &self.child_docs,
                        child_types,
                        offset,
                        index: 0,
                    })
            }
            _ => None,
        }
    }
}

pub fn field_name_to_cow<'a>(name: FieldName<'a>) -> Cow<'a, str> {
    match name {
        FieldName::Owned(s) => Cow::Owned(s),
        FieldName::Borrowed(s) => Cow::Borrowed(s),
        FieldName::Static(s) => Cow::Borrowed(s),
    }
}

// =============================================================================
// ChildField — borrowed view of a child within a parent's name storage
// =============================================================================

/// A borrowed view of a single child field within a Group (or nested structure).
///
/// Holds references into the parent's `FieldNames` rather than copying.
/// Supports recursive descent into nested Groups.
#[derive(Debug)]
pub struct ChildField<'f> {
    pub(crate) name: &'f str,
    pub(crate) doc: &'f str,
    pub(crate) child_names: &'f FieldNames<'f>,
    pub(crate) child_docs: &'f FieldNames<'f>,
    pub(crate) child_names_offset: usize,
    pub(crate) child_names_len: usize,
    pub(crate) datatype: &'f PyroType,
}

impl<'f> ChildField<'f> {
    #[inline]
    pub fn name(&self) -> &'f str {
        self.name
    }

    #[inline]
    pub fn doc(&self) -> &'f str {
        self.doc
    }

    #[inline]
    pub fn datatype(&self) -> &'f PyroType {
        self.datatype
    }

    /// Number of flattened descendant name slots this field's type owns —
    /// i.e. `self.datatype().name_count()`. Indexes into
    /// [`Self::child_name`] / [`Self::child_doc`] run `0..this`.
    #[inline]
    pub fn child_names_len(&self) -> usize {
        self.child_names_len
    }

    /// The `index`-th flattened descendant name, in the same pre-order
    /// packing [`PyroField::child_names`] uses.
    #[inline]
    pub fn child_name(&self, index: usize) -> &'f str {
        self.child_names.get(self.child_names_offset + index)
    }

    /// The `index`-th flattened descendant doc string, parallel to
    /// [`Self::child_name`].
    #[inline]
    pub fn child_doc(&self, index: usize) -> &'f str {
        self.child_docs.get(self.child_names_offset + index)
    }

    pub fn num_children(&self) -> usize {
        match self.datatype {
            PyroType::Group(ct) | PyroType::OptionGroup(ct) => ct.len(),
            _ => 0,
        }
    }

    /// Get the i-th child of this (nested) Group.
    pub fn group_child(&self, index: usize) -> ChildField<'f> {
        let child_types = match self.datatype {
            PyroType::Group(ct) | PyroType::OptionGroup(ct) => ct,
            _ => panic!("group_child() on non-Group: {:?}", self.datatype),
        };
        assert!(index < child_types.len());

        let mut rel_offset = 0;
        for i in 0..index {
            rel_offset += 1 + child_types[i].name_count();
        }
        let abs_offset = self.child_names_offset + rel_offset;
        let nc = child_types[index].name_count();

        ChildField {
            name: self.child_names.get(abs_offset),
            doc: self.child_docs.get(abs_offset),
            child_names: self.child_names,
            child_docs: self.child_docs,
            child_names_offset: abs_offset + 1,
            child_names_len: nc,
            datatype: &child_types[index],
        }
    }

    /// Iterate children of this (nested) Group.
    pub fn group_children(&self) -> Option<ChildIter<'f>> {
        match self.datatype {
            PyroType::Group(child_types) | PyroType::OptionGroup(child_types) => Some(ChildIter {
                parent_names: self.child_names,
                parent_docs: self.child_docs,
                child_types,
                offset: self.child_names_offset,
                index: 0,
            }),
            _ => None,
        }
    }

    /// If this is one or more `List` layers wrapping a `Group`/`OptionGroup`,
    /// iterate inner group's children.
    pub fn list_group_children(&self) -> Option<ChildIter<'f>> {
        let (nesting, _, child_types) = self.datatype.list_group_nesting()?;
        if nesting == 0 {
            return None;
        }
        Some(ChildIter {
            parent_names: self.child_names,
            parent_docs: self.child_docs,
            child_types,
            offset: self.child_names_offset,
            index: 0,
        })
    }

    /// If this is `Map { key: Group(...), .. }`, iterate the key group's children.
    pub fn map_key_group_children(&self) -> Option<ChildIter<'f>> {
        match self.datatype {
            PyroType::Map { key, .. } => {
                key.list_group_children_types()
                    .map(|child_types| ChildIter {
                        parent_names: self.child_names,
                        parent_docs: self.child_docs,
                        child_types,
                        offset: self.child_names_offset,
                        index: 0,
                    })
            }
            _ => None,
        }
    }

    /// If this is `Map { value: Group(...), .. }`, iterate the value group's children.
    pub fn map_value_group_children(&self) -> Option<ChildIter<'f>> {
        match self.datatype {
            PyroType::Map { key, value } => {
                let offset = self.child_names_offset + key.name_count();
                value
                    .list_group_children_types()
                    .map(|child_types| ChildIter {
                        parent_names: self.child_names,
                        parent_docs: self.child_docs,
                        child_types,
                        offset,
                        index: 0,
                    })
            }
            _ => None,
        }
    }

    /// Materialize into an owned `PyroField<'static>`.
    pub fn to_owned_field(&self) -> PyroField<'static> {
        let child_names: Vec<Cow<'static, str>> = (0..self.child_names_len)
            .map(|i| Cow::Owned(self.child_names.get(self.child_names_offset + i).to_owned()))
            .collect();
        let child_docs: Vec<Cow<'static, str>> = (0..self.child_names_len)
            .map(|i| Cow::Owned(self.child_docs.get(self.child_names_offset + i).to_owned()))
            .collect();
        PyroField {
            name: FieldName::Owned(self.name.to_owned()),
            doc: FieldName::Owned(self.doc.to_owned()),
            child_names: FieldNames::Owned(child_names),
            child_docs: FieldNames::Owned(child_docs),
            datatype: self.datatype.clone(),
        }
    }
}

// =============================================================================
// ChildIter
// =============================================================================

/// Iterates over children of a Group, yielding `ChildField` views.
pub struct ChildIter<'f> {
    pub(crate) parent_names: &'f FieldNames<'f>,
    pub(crate) parent_docs: &'f FieldNames<'f>,
    pub(crate) child_types: &'f [PyroType],
    pub(crate) offset: usize,
    pub(crate) index: usize,
}

impl<'f> Iterator for ChildIter<'f> {
    type Item = ChildField<'f>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.child_types.len() {
            return None;
        }
        let child_type = &self.child_types[self.index];
        let nc = child_type.name_count();

        let name = self.parent_names.get(self.offset);
        let doc = self.parent_docs.get(self.offset);
        let child_offset = self.offset + 1;

        self.offset += 1 + nc;
        self.index += 1;

        Some(ChildField {
            name,
            doc,
            child_names: self.parent_names,
            child_docs: self.parent_docs,
            child_names_offset: child_offset,
            child_names_len: nc,
            datatype: child_type,
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let r = self.child_types.len() - self.index;
        (r, Some(r))
    }
}

impl<'f> ExactSizeIterator for ChildIter<'f> {}
