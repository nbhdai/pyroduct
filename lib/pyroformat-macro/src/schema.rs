//! A Rust type as a [`PyroType`], for describing an interface from its
//! source rather than from compiled code.
//!
//! Field shapes come from the same [`type_map`](crate::parse) the derives lay
//! out from, so a spec says what the wire carries. A struct a field names is
//! looked up among the [`StructInput`]s registered with the builder, then
//! among the [`PyroSchema`]s of dependencies; one found in neither is
//! [`PyroType::Unknown`].

use std::borrow::Cow;
use std::collections::HashMap;

use pyro_schema::{FieldNames, PrimitiveDataType, PyroField, PyroSchema, PyroType};
use syn::{Error, Result, Type};

use crate::parse::{Leaf, Prim, ResolvedType, StructInput, type_map, wire_type};

/// Every struct an interface's types may name.
#[derive(Default)]
pub struct SchemaBuilder {
    structs: HashMap<String, StructInput>,
    foreign: HashMap<String, PyroSchema<'static>>,
}

/// A resolved type with the names and docs of every group it reaches, in the
/// pre-order [`PyroField`] stores them in.
struct Parts {
    ty: PyroType,
    names: Vec<Cow<'static, str>>,
    docs: Vec<Cow<'static, str>>,
}

impl Parts {
    fn leaf(ty: PyroType) -> Self {
        Parts {
            ty,
            names: Vec::new(),
            docs: Vec::new(),
        }
    }

    /// Under `depth` more list levels.
    fn listed(mut self, depth: usize) -> Self {
        for _ in 0..depth {
            self.ty = PyroType::List(pyro_schema::Bow::new(self.ty));
        }
        self
    }
}

impl SchemaBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// A struct of this interface's own.
    pub fn insert(&mut self, input: StructInput) {
        self.structs.insert(input.name.to_string(), input);
    }

    /// A struct a dependency's interface already describes.
    pub fn insert_foreign(&mut self, name: impl Into<String>, schema: PyroSchema<'static>) {
        self.foreign.insert(name.into(), schema);
    }

    /// The names of this interface's own structs.
    pub fn struct_names(&self) -> impl Iterator<Item = &str> {
        self.structs.keys().map(String::as_str)
    }

    /// The schema of one of this interface's own structs, by name.
    pub fn schema_for(&self, name: &str) -> Result<Option<PyroSchema<'static>>> {
        self.structs
            .get(name)
            .map(|input| self.schema_of(input))
            .transpose()
    }

    /// `input`'s fields as a schema, its own doc as the schema's.
    pub fn schema_of(&self, input: &StructInput) -> Result<PyroSchema<'static>> {
        let mut visited = vec![input.name.to_string()];
        let fields = input
            .fields
            .iter()
            .map(|f| {
                let parts = self.resolve(&f.resolved, &mut visited)?;
                Ok(Self::field_from_parts(f.ident.to_string(), &f.doc, parts))
            })
            .collect::<Result<Vec<_>>>()?;
        let mut schema = PyroSchema::new(fields).with_doc(input.doc.clone());
        schema.set_word(input.word);
        Ok(schema)
    }

    /// A field of type `ty`, as written: a borrowed type is described by
    /// the owned [`wire_type`] it travels as.
    pub fn field(&self, name: &str, ty: &Type, doc: &str) -> Result<PyroField<'static>> {
        let resolved = type_map::resolve_type(&wire_type(ty)?, false)?;
        let parts = self.resolve(&resolved, &mut Vec::new())?;
        Ok(Self::field_from_parts(name.to_string(), doc, parts))
    }

    fn field_from_parts(name: String, doc: &str, parts: Parts) -> PyroField<'static> {
        PyroField::with_children_and_docs(
            name,
            FieldNames::Owned(parts.names),
            FieldNames::Owned(parts.docs),
            parts.ty,
        )
        .with_doc(doc.to_string())
    }

    fn resolve(&self, resolved: &ResolvedType, visited: &mut Vec<String>) -> Result<Parts> {
        Ok(match resolved {
            ResolvedType::Leaf { leaf, vec_depth } => {
                let (ty, depth) = match *leaf {
                    // A plain primitive list is its own first list level.
                    Leaf::Prim(p) if *vec_depth > 0 => {
                        (PyroType::PrimitiveList(prim(p)), vec_depth - 1)
                    }
                    Leaf::Prim(p) => (PyroType::PrimitiveScalar(prim(p)), 0),
                    Leaf::OptionPrim(p) => (PyroType::OptionPrimitiveScalar(prim(p)), *vec_depth),
                    Leaf::Array(n, p) => (PyroType::PrimitiveFixedList(prim(p), n), *vec_depth),
                    Leaf::OptionArray(n, p) => {
                        (PyroType::OptionPrimitiveFixedList(prim(p), n), *vec_depth)
                    }
                    Leaf::Str => (PyroType::Str, *vec_depth),
                    Leaf::Bytes => (PyroType::Bytes, *vec_depth),
                    Leaf::BitVec => (PyroType::PrimitiveList(PrimitiveDataType::Bool), *vec_depth),
                };
                Parts::leaf(ty).listed(depth)
            }
            ResolvedType::Map {
                key,
                value,
                vec_depth,
            } => {
                let key = self.resolve(key, visited)?;
                let value = self.resolve(value, visited)?;
                let mut names = key.names;
                names.extend(value.names);
                let mut docs = key.docs;
                docs.extend(value.docs);
                Parts {
                    ty: PyroType::Map {
                        key: pyro_schema::Bow::new(key.ty),
                        value: pyro_schema::Bow::new(value.ty),
                    },
                    names,
                    docs,
                }
                .listed(*vec_depth)
            }
            ResolvedType::Struct {
                ty,
                vec_depth,
                optional,
            } => self
                .resolve_struct(ty, *optional, visited)?
                .listed(*vec_depth),
        })
    }

    /// A named struct as a group: one of ours, else a dependency's. A
    /// reader view (`FooRef<'a>`) describes the struct it views.
    fn resolve_struct(
        &self,
        ty: &Type,
        optional: bool,
        visited: &mut Vec<String>,
    ) -> Result<Parts> {
        let Some(name) = struct_name(ty) else {
            return Ok(Parts::leaf(PyroType::Unknown));
        };
        let base = name.strip_suffix("Ref").unwrap_or(&name);
        let name = if self.structs.contains_key(&name) || self.foreign.contains_key(&name) {
            name.clone()
        } else {
            base.to_string()
        };

        let group = |children: Vec<PyroType>| {
            if optional {
                PyroType::OptionGroup(children.into())
            } else {
                PyroType::Group(children.into())
            }
        };

        if let Some(input) = self.structs.get(&name) {
            if visited.contains(&name) {
                return Err(Error::new_spanned(
                    ty,
                    format!("`{name}` contains itself, which the wire can't describe"),
                ));
            }
            visited.push(name);
            let mut parts = Parts::leaf(PyroType::Unknown);
            let mut children = Vec::with_capacity(input.fields.len());
            for f in &input.fields {
                let child = self.resolve(&f.resolved, visited)?;
                parts.names.push(Cow::Owned(f.ident.to_string()));
                parts.docs.push(Cow::Owned(f.doc.clone()));
                parts.names.extend(child.names);
                parts.docs.extend(child.docs);
                children.push(child.ty);
            }
            visited.pop();
            parts.ty = group(children);
            Ok(parts)
        } else if let Some(schema) = self.foreign.get(&name) {
            Ok(Parts {
                ty: group(schema.types().to_vec()),
                names: schema
                    .names()
                    .iter()
                    .map(|n| Cow::Owned(n.to_string()))
                    .collect(),
                docs: schema
                    .field_docs()
                    .iter()
                    .map(|d| Cow::Owned(d.to_string()))
                    .collect(),
            })
        } else {
            Ok(Parts::leaf(PyroType::Unknown))
        }
    }
}

fn struct_name(ty: &Type) -> Option<String> {
    match ty {
        Type::Group(group) => struct_name(&group.elem),
        Type::Paren(paren) => struct_name(&paren.elem),
        Type::Path(path) => path.path.segments.last().map(|s| s.ident.to_string()),
        _ => None,
    }
}

fn prim(p: Prim) -> PrimitiveDataType {
    match p {
        Prim::U8 => PrimitiveDataType::U8,
        Prim::I8 => PrimitiveDataType::I8,
        Prim::U16 => PrimitiveDataType::U16,
        Prim::I16 => PrimitiveDataType::I16,
        Prim::U32 => PrimitiveDataType::U32,
        Prim::I32 => PrimitiveDataType::I32,
        Prim::U64 => PrimitiveDataType::U64,
        Prim::I64 => PrimitiveDataType::I64,
        Prim::F16 => PrimitiveDataType::F16,
        Prim::F32 => PrimitiveDataType::F32,
        Prim::F64 => PrimitiveDataType::F64,
        Prim::Bool => PrimitiveDataType::Bool,
        Prim::Time => PrimitiveDataType::Timestamp,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    fn builder() -> SchemaBuilder {
        let mut b = SchemaBuilder::new();
        b.insert(
            StructInput::parse(&parse_quote! {
                /// A point
                struct Point {
                    /// Across
                    x: f32,
                    y: f32,
                }
            })
            .unwrap(),
        );
        b
    }

    #[test]
    fn leaves_match_the_wire() {
        let b = builder();
        let ty = |t: Type| b.field("f", &t, "").unwrap().datatype().clone();
        let list = |t| PyroType::List(pyro_schema::Bow::new(t));

        assert_eq!(
            ty(parse_quote!(u32)),
            PyroType::PrimitiveScalar(PrimitiveDataType::U32)
        );
        assert_eq!(ty(parse_quote!(&str)), PyroType::Str);
        assert_eq!(ty(parse_quote!(&[u8])), PyroType::Bytes);
        assert_eq!(
            ty(parse_quote!(Vec<f64>)),
            PyroType::PrimitiveList(PrimitiveDataType::F64)
        );
        assert_eq!(
            ty(parse_quote!(Vec<Vec<f64>>)),
            list(PyroType::PrimitiveList(PrimitiveDataType::F64))
        );
        assert_eq!(ty(parse_quote!(Vec<String>)), list(PyroType::Str));
        assert_eq!(
            ty(parse_quote!(Option<u8>)),
            PyroType::OptionPrimitiveScalar(PrimitiveDataType::U8)
        );
        assert_eq!(
            ty(parse_quote!([i16; 3])),
            PyroType::PrimitiveFixedList(PrimitiveDataType::I16, 3)
        );
        assert_eq!(ty(parse_quote!(Missing)), PyroType::Unknown);
    }

    #[test]
    fn structs_expand_with_names_and_docs() {
        let b = builder();
        let point = PyroType::Group(
            vec![
                PyroType::PrimitiveScalar(PrimitiveDataType::F32),
                PyroType::PrimitiveScalar(PrimitiveDataType::F32),
            ]
            .into(),
        );

        let field = b.field("path", &parse_quote!(Vec<Point>), "").unwrap();
        assert_eq!(
            field.datatype(),
            &PyroType::List(pyro_schema::Bow::new(point.clone()))
        );

        let field = b
            .field("at", &parse_quote!(Option<PointRef<'_>>), "")
            .unwrap();
        assert!(matches!(field.datatype(), PyroType::OptionGroup(_)));

        let schema = b.schema_for("Point").unwrap().unwrap();
        assert_eq!(schema.doc(), "A point");
        let names: Vec<_> = schema
            .fields()
            .map(|f| (f.name().to_string(), f.doc().to_string()))
            .collect();
        assert_eq!(
            names,
            [("x".into(), "Across".into()), ("y".into(), String::new())]
        );

        let mut dep = SchemaBuilder::new();
        dep.insert_foreign("Point", schema);
        let field = dep.field("p", &parse_quote!(Point), "").unwrap();
        assert_eq!(field.datatype(), &point);
    }
}
