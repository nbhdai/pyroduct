//! File-level schema builder.
//!
//! Parses every struct in a capability source file to build a registry of
//! `name → PyroSchema`, then resolves field types against that registry so
//! that nested user-defined structs produce correct `Group(fields)` instead of
//! opaque empty groups.
//!
//! # Usage
//!
//! ```ignore
//! let builder = SchemaBuilder::from_file(&syn_file);
//! let pyro_type = builder.resolve_type(&syn_ty);
//! let schema    = builder.schema_for("MyStruct").unwrap();
//! ```

use std::borrow::Cow;
use std::collections::HashMap;

use pyro_spec::{PrimitiveDataType, PyroField, PyroSchema, PyroType};
use syn::{Attribute, Expr, Fields, Lit, Meta};
use crate::utils::has_attr;

// =============================================================================
// SchemaBuilder
// =============================================================================

/// A file-level registry of struct schemas.
///
/// Built from a parsed `syn::File`, it maps every struct name to its fields
/// (name, syn::Type, doc-string). The registry is then used to resolve
/// `syn::Type` → `PyroType` with full knowledge of sibling structs.
pub struct SchemaBuilder {
    /// Struct name → list of (field_name, field_type, field_doc).
    structs: HashMap<String, StructEntry>,
}

struct StructEntry {
    doc: Option<String>,
    fields: Vec<FieldEntry>,
}

struct FieldEntry {
    name: String,
    ty: syn::Type,
    doc: Option<String>,
}

impl SchemaBuilder {
    // -----------------------------------------------------------------
    // Construction
    // -----------------------------------------------------------------

    /// Scan every `Item::Struct` in the file and register it.
    pub fn from_file(file: &syn::File) -> Self {
        let mut structs = HashMap::new();
        for item in &file.items {
            if let syn::Item::Struct(s) = item {
                if !(has_attr(&s.attrs, "config") || has_attr(&s.attrs, "magma")) {
                    continue;
                }
                let name = s.ident.to_string();
                let doc = extract_doc_string(&s.attrs);
                let fields = Self::collect_fields(&s.fields);
                structs.insert(name, StructEntry { doc, fields });
            }
        }
        Self { structs }
    }

    fn collect_fields(fields: &Fields) -> Vec<FieldEntry> {
        match fields {
            Fields::Named(named) => named
                .named
                .iter()
                .map(|f| FieldEntry {
                    name: f.ident.as_ref().unwrap().to_string(),
                    ty: f.ty.clone(),
                    doc: extract_doc_string(&f.attrs),
                })
                .collect(),
            Fields::Unnamed(unnamed) => unnamed
                .unnamed
                .iter()
                .enumerate()
                .map(|(i, f)| FieldEntry {
                    name: i.to_string(),
                    ty: f.ty.clone(),
                    doc: extract_doc_string(&f.attrs),
                })
                .collect(),
            Fields::Unit => vec![],
        }
    }

    // -----------------------------------------------------------------
    // Resolution
    // -----------------------------------------------------------------

    /// Build a `PyroSchema` for a struct that is in the registry.
    pub fn schema_for(&self, struct_name: &str) -> Option<PyroSchema<'static>> {
        let entry = self.structs.get(struct_name)?;
        let mut visited = Vec::new();
        let fields = self.resolve_fields_inner(&entry.fields, &mut visited);
        let mut schema = PyroSchema::new(fields);
        if let Some(d) = &entry.doc {
            schema = schema.add_docstring(Cow::Owned(d.clone()));
        }
        Some(schema)
    }

    /// Resolve a `syn::Type` to a `PyroType`, expanding known struct names
    /// into full `Group(fields)`.
    pub fn resolve_type(&self, ty: &syn::Type) -> PyroType<'static> {
        self.resolve_type_inner(ty, &mut Vec::new())
    }

    /// Check whether a `syn::Type` is `Option<_>`.
    pub fn is_option(ty: &syn::Type) -> bool {
        is_option_type(ty)
    }

    // -----------------------------------------------------------------
    // Internals
    // -----------------------------------------------------------------

    fn resolve_fields_inner(
        &self,
        fields: &[FieldEntry],
        visited: &mut Vec<String>,
    ) -> Vec<PyroField<'static>> {
        fields
            .iter()
            .map(|f| {
                let data_type = self.resolve_type_inner(&f.ty, visited);
                let nullable = is_option_type(&f.ty);
                let mut field = PyroField::new(Cow::Owned(f.name.clone()), data_type, nullable);
                if let Some(doc) = &f.doc {
                    field = field.add_docstring(Cow::Owned(doc.clone()));
                }
                field
            })
            .collect()
    }

    /// Core resolver. `visited` tracks struct names on the current path to
    /// break infinite recursion from cyclic types (which shouldn't occur in
    /// practice but we guard against it).
    fn resolve_type_inner(&self, ty: &syn::Type, visited: &mut Vec<String>) -> PyroType<'static> {
        match ty {
            syn::Type::Path(type_path) => {
                let segment = match type_path.path.segments.last() {
                    Some(s) => s,
                    None => return PyroType::Null,
                };
                let ident_str = segment.ident.to_string();

                match ident_str.as_str() {
                    // --- Primitives ---
                    "bool" => PyroType::PrimitiveScalar(PrimitiveDataType::Bool),
                    "u8" => PyroType::PrimitiveScalar(PrimitiveDataType::U8),
                    "u16" => PyroType::PrimitiveScalar(PrimitiveDataType::U16),
                    "u32" => PyroType::PrimitiveScalar(PrimitiveDataType::U32),
                    "u64" => PyroType::PrimitiveScalar(PrimitiveDataType::U64),
                    "i8" => PyroType::PrimitiveScalar(PrimitiveDataType::I8),
                    "i16" => PyroType::PrimitiveScalar(PrimitiveDataType::I16),
                    "i32" => PyroType::PrimitiveScalar(PrimitiveDataType::I32),
                    "i64" => PyroType::PrimitiveScalar(PrimitiveDataType::I64),
                    "f16" => PyroType::PrimitiveScalar(PrimitiveDataType::F16),
                    "f32" => PyroType::PrimitiveScalar(PrimitiveDataType::F32),
                    "f64" => PyroType::PrimitiveScalar(PrimitiveDataType::F64),

                    // --- Strings ---
                    "String" | "str" => PyroType::Str,

                    // --- Bytes ---
                    "Bytes" => PyroType::PrimitiveList(PrimitiveDataType::U8),

                    // --- Option<T> ---
                    "Option" => {
                        if let Some(inner) = extract_single_generic_arg(segment) {
                            self.resolve_type_inner(inner, visited)
                        } else {
                            PyroType::Null
                        }
                    }

                    // --- Vec<T> ---
                    "Vec" => {
                        if let Some(inner) = extract_single_generic_arg(segment) {
                            let inner_pyro = self.resolve_type_inner(inner, visited);
                            match &inner_pyro {
                                PyroType::PrimitiveScalar(p) => PyroType::PrimitiveList(*p),
                                _ => PyroType::List(Box::new(inner_pyro), false),
                            }
                        } else {
                            PyroType::Null
                        }
                    }

                    // --- HashMap / BTreeMap ---
                    "HashMap" | "BTreeMap" => {
                        if let Some((k, v)) = extract_two_generic_args(segment) {
                            PyroType::Map {
                                key: Box::new(self.resolve_type_inner(k, visited)),
                                value: Box::new(self.resolve_type_inner(v, visited)),
                            }
                        } else {
                            PyroType::Null
                        }
                    }

                    // --- Result<T, E> ---
                    "Result" => {
                        if let Some((ok, _err)) = extract_two_generic_args(segment) {
                            self.resolve_type_inner(ok, visited)
                        } else {
                            PyroType::Null
                        }
                    }

                    // --- DateTime ---
                    "DateTime" => PyroType::Timestamp,

                    // --- User-defined struct (look up in registry) ---
                    other => {
                        if visited.contains(&other.to_string()) {
                            // Cycle guard — return empty group
                            return PyroType::Group(Cow::Owned(vec![]));
                        }
                        if let Some(entry) = self.structs.get(other) {
                            visited.push(other.to_string());
                            let fields = self.resolve_fields_inner(&entry.fields, visited);
                            visited.pop();
                            PyroType::Group(Cow::Owned(fields))
                        } else {
                            // Unknown struct — opaque group
                            PyroType::Group(Cow::Owned(vec![]))
                        }
                    }
                }
            }
            syn::Type::Reference(r) => self.resolve_type_inner(&r.elem, visited),
            syn::Type::Tuple(t) if t.elems.is_empty() => PyroType::Null,
            _ => PyroType::Null,
        }
    }
}

// =============================================================================
// Helpers (private)
// =============================================================================

fn is_option_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        if let Some(seg) = type_path.path.segments.last() {
            return seg.ident == "Option";
        }
    }
    false
}

fn extract_single_generic_arg(segment: &syn::PathSegment) -> Option<&syn::Type> {
    if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
        if let Some(syn::GenericArgument::Type(ty)) = args.args.first() {
            return Some(ty);
        }
    }
    None
}

fn extract_two_generic_args(segment: &syn::PathSegment) -> Option<(&syn::Type, &syn::Type)> {
    if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
        let mut iter = args.args.iter();
        if let (Some(syn::GenericArgument::Type(a)), Some(syn::GenericArgument::Type(b))) =
            (iter.next(), iter.next())
        {
            return Some((a, b));
        }
    }
    None
}

fn extract_doc_string(attrs: &[Attribute]) -> Option<String> {
    let mut lines = Vec::new();
    for attr in attrs {
        if attr.path().is_ident("doc") {
            if let Meta::NameValue(nv) = &attr.meta {
                if let Expr::Lit(expr_lit) = &nv.value {
                    if let Lit::Str(lit_str) = &expr_lit.lit {
                        lines.push(lit_str.value().trim().to_string());
                    }
                }
            }
        }
    }
    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n"))
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;
    use syn::parse2;

    fn builder_from_tokens(tokens: proc_macro2::TokenStream) -> SchemaBuilder {
        let file: syn::File = syn::parse2(tokens).unwrap();
        SchemaBuilder::from_file(&file)
    }

    // --- Primitive resolution ---

    #[test]
    fn test_resolve_primitives() {
        let builder = builder_from_tokens(quote! {});

        let ty: syn::Type = parse2(quote!(u32)).unwrap();
        assert_eq!(
            builder.resolve_type(&ty),
            PyroType::PrimitiveScalar(PrimitiveDataType::U32)
        );

        let ty: syn::Type = parse2(quote!(String)).unwrap();
        assert_eq!(builder.resolve_type(&ty), PyroType::Str);

        let ty: syn::Type = parse2(quote!(f64)).unwrap();
        assert_eq!(
            builder.resolve_type(&ty),
            PyroType::PrimitiveScalar(PrimitiveDataType::F64)
        );
    }

    // --- Vec / Option ---

    #[test]
    fn test_resolve_vec_and_option() {
        let builder = builder_from_tokens(quote! {});

        let ty: syn::Type = parse2(quote!(Vec<u8>)).unwrap();
        assert_eq!(
            builder.resolve_type(&ty),
            PyroType::PrimitiveList(PrimitiveDataType::U8)
        );

        let ty: syn::Type = parse2(quote!(Vec<String>)).unwrap();
        assert_eq!(
            builder.resolve_type(&ty),
            PyroType::List(Box::new(PyroType::Str), false)
        );

        let ty: syn::Type = parse2(quote!(Option<i32>)).unwrap();
        assert_eq!(
            builder.resolve_type(&ty),
            PyroType::PrimitiveScalar(PrimitiveDataType::I32)
        );
        assert!(SchemaBuilder::is_option(&ty));
    }

    // --- Nested struct resolution (the whole point) ---

    #[test]
    fn test_resolve_nested_struct() {
        let builder = builder_from_tokens(quote! {
            #[config]
            struct Foo {
                woobie: String,
            }

            #[config]
            struct Bar {
                doobie: Foo,
            }
        });

        // Foo resolves to Group with one Str field
        let ty_foo: syn::Type = parse2(quote!(Foo)).unwrap();
        assert_eq!(
            builder.resolve_type(&ty_foo),
            PyroType::Group(Cow::Owned(vec![PyroField::new(
                Cow::Borrowed("woobie"),
                PyroType::Str,
                false,
            )]))
        );

        // Bar resolves to Group with one Group field (the Foo)
        let schema = builder.schema_for("Bar").unwrap();
        assert_eq!(schema.fields.len(), 1);

        let doobie = &schema.fields()[0];
        assert_eq!(doobie.name(), "doobie");
        match &doobie.data_type {
            PyroType::Group(inner_fields) => {
                assert_eq!(inner_fields.len(), 1);
                assert_eq!(inner_fields[0].name(), "woobie");
                assert_eq!(inner_fields[0].data_type, PyroType::Str);
            }
            other => panic!("expected Group, got {:?}", other),
        }
    }

    // --- Deeply nested ---

    #[test]
    fn test_resolve_deeply_nested() {
        let builder = builder_from_tokens(quote! {
            #[config]
            struct A {
                x: i32,
            }
            #[config]
            struct B {
                a: A,
                name: String,
            }
            #[config]
            struct C {
                b: B,
                flag: bool,
            }
        });

        let schema_c = builder.schema_for("C").unwrap();
        assert_eq!(schema_c.fields.len(), 2);

        // b field should be Group([ Group([x:I32]), name:Str ])
        let b_field = &schema_c.fields()[0];
        assert_eq!(b_field.name(), "b");
        match &b_field.data_type {
            PyroType::Group(b_fields) => {
                assert_eq!(b_fields.len(), 2);
                assert_eq!(b_fields[0].name(), "a");
                match &b_fields[0].data_type {
                    PyroType::Group(a_fields) => {
                        assert_eq!(a_fields.len(), 1);
                        assert_eq!(a_fields[0].name(), "x");
                        assert_eq!(
                            a_fields[0].data_type,
                            PyroType::PrimitiveScalar(PrimitiveDataType::I32)
                        );
                    }
                    other => panic!("expected Group for A, got {:?}", other),
                }
                assert_eq!(b_fields[1].name(), "name");
                assert_eq!(b_fields[1].data_type, PyroType::Str);
            }
            other => panic!("expected Group for B, got {:?}", other),
        }

        // flag field
        let flag_field = &schema_c.fields()[1];
        assert_eq!(flag_field.name(), "flag");
        assert_eq!(
            flag_field.data_type,
            PyroType::PrimitiveScalar(PrimitiveDataType::Bool)
        );
    }

    // --- Vec of struct ---

    #[test]
    fn test_resolve_vec_of_struct() {
        let builder = builder_from_tokens(quote! {
            #[config]
            struct Item {
                value: f32,
            }
            #[config]
            struct Container {
                items: Vec<Item>,
            }
        });

        let schema = builder.schema_for("Container").unwrap();
        let items_field = &schema.fields()[0];
        assert_eq!(items_field.name(), "items");

        match &items_field.data_type {
            PyroType::List(inner, nullable) => {
                assert!(!nullable);
                match inner.as_ref() {
                    PyroType::Group(fields) => {
                        assert_eq!(fields.len(), 1);
                        assert_eq!(fields[0].name(), "value");
                        assert_eq!(
                            fields[0].data_type,
                            PyroType::PrimitiveScalar(PrimitiveDataType::F32)
                        );
                    }
                    other => panic!("expected Group inside List, got {:?}", other),
                }
            }
            other => panic!("expected List, got {:?}", other),
        }
    }

    // --- Doc strings preserved ---

    #[test]
    fn test_doc_strings_preserved() {
        let builder = builder_from_tokens(quote! {
            /// This is Foo
            #[config]
            struct Foo {
                /// The id
                id: u32,
                name: String,
            }
        });

        let schema = builder.schema_for("Foo").unwrap();
        assert_eq!(schema.documentation.as_deref(), Some("This is Foo"));
        assert_eq!(schema.fields.len(), 2);
        assert_eq!(schema.fields()[0].documentation.as_deref(), Some("The id"));
        assert!(schema.fields()[1].documentation.is_none());
    }

    // --- Unknown struct falls back to empty group ---

    #[test]
    fn test_unknown_struct_empty_group() {
        let builder = builder_from_tokens(quote! {
            #[config]
            struct Wrapper {
                inner: SomeExternalThing,
            }
        });

        let schema = builder.schema_for("Wrapper").unwrap();
        let inner = &schema.fields()[0];
        assert_eq!(inner.data_type, PyroType::Group(Cow::Owned(vec![])));
    }

    // --- Cycle guard ---

    #[test]
    fn test_cycle_guard() {
        // Contrived: struct A has field of type A. Should not stack overflow.
        let builder = builder_from_tokens(quote! {
            #[config]
            struct A {
                next: A,
            }
        });

        let schema = builder.schema_for("A").unwrap();
        assert_eq!(schema.fields().len(), 1);
        let next_field = &schema.fields()[0];
        assert_eq!(next_field.name(), "next");

        // The `next` field has type A, which is a Group containing one field also named `next`.
        // The inner self-reference is cut off as Group([]) to break the cycle.
        match &next_field.data_type {
            PyroType::Group(inner_fields) => {
                assert_eq!(inner_fields.len(), 1);
                assert_eq!(inner_fields[0].name(), "next");
                // Cycle broken at this depth — the recursive self-ref is an empty group
                assert_eq!(
                    inner_fields[0].data_type,
                    PyroType::Group(Cow::Owned(vec![]))
                );
            }
            other => panic!("expected Group for A's next field, got {:?}", other),
        }
    }

    // --- Map of struct ---

    #[test]
    fn test_resolve_map_of_struct() {
        let builder = builder_from_tokens(quote! {
            #[config]
            struct Config {
                key: String,
            }
            #[config]
            struct Registry {
                entries: HashMap<String, Config>,
            }
        });

        let schema = builder.schema_for("Registry").unwrap();
        let entries = &schema.fields()[0];

        match &entries.data_type {
            PyroType::Map { key, value } => {
                assert_eq!(key.as_ref(), &PyroType::Str);
                match value.as_ref() {
                    PyroType::Group(fields) => {
                        assert_eq!(fields.len(), 1);
                        assert_eq!(fields[0].name(), "key");
                    }
                    other => panic!("expected Group for Config, got {:?}", other),
                }
            }
            other => panic!("expected Map, got {:?}", other),
        }
    }
}
