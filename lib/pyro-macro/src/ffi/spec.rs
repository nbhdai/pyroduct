//! Generates interface specifications using the Pyro type system.
//!
//! The spec is built from `pyro_schema::interface`'s types. Type resolution
//! is delegated to [`SchemaBuilder`], which knows every struct in the source
//! file (and in the interfaces it depends on), so nested user-defined types
//! expand into proper `Group`s with their field names.

use std::collections::BTreeMap;

use heck::AsSnakeCase;
use pyro_schema::interface::{CapabilityFunc, ClassSpec, InterfaceSpec};
use pyro_schema::{FieldName, PyroSchema};
use pyroformat_macro::parse::StructInput;
use pyroformat_macro::schema::SchemaBuilder;
use syn::{Attribute, Expr, Lit, Meta, Type};

use crate::utils::has_attr;

use super::capability::CapabilityImpl;

// =============================================================================
// SpecBuilder
// =============================================================================

/// Every wire struct declared in `file` (`#[config]`, `#[magma]`, or
/// `#[derive(Layout)]`), plus the structs `dep_interfaces` declare.
pub fn schema_builder(
    file: &syn::File,
    dep_interfaces: &[InterfaceSpec<'static>],
) -> syn::Result<SchemaBuilder> {
    let mut builder = SchemaBuilder::new();
    for spec in dep_interfaces {
        for (name, schema) in &spec.structs {
            builder.insert_foreign(name.to_string(), schema.clone());
        }
    }
    for item in &file.items {
        if let syn::Item::Struct(s) = item
            && (has_attr(&s.attrs, "config")
                || has_attr(&s.attrs, "magma")
                || derives_layout(&s.attrs))
        {
            builder.insert(StructInput::parse(&s.clone().into())?);
        }
    }
    Ok(builder)
}

pub fn build_class_spec(
    cap: &CapabilityImpl,
    builder: &SchemaBuilder,
) -> syn::Result<ClassSpec<'static>> {
    let capability = AsSnakeCase(cap.ident.state_tn.to_string()).to_string();
    let description = extract_doc_string(&cap.attrs);

    let methods = cap
        .methods
        .iter()
        .map(|m| {
            Ok(CapabilityFunc {
                name: m.name.to_string().into(),
                description: extract_doc_string(&m.attrs).map(Into::into),
                input: builder.schema_of(&m.inputs)?,
                output: output_schema(&m.output.ok_type, builder)?,
            })
        })
        .collect::<syn::Result<Vec<_>>>()?;

    let client = builder.schema_for(&cap.ident.client_tn.to_string())?;
    let config = match &cap.ident.config_tn {
        Some(config_tn) => builder.schema_for(&config_tn.to_string())?,
        None => None,
    };

    Ok(ClassSpec {
        name: capability.into(),
        description: description.map(Into::into),
        client,
        config,
        methods,
    })
}

pub fn build_spec(name: &str, file: &syn::File) -> syn::Result<InterfaceSpec<'static>> {
    let builder = schema_builder(file, &[])?;

    let mut classes: Vec<ClassSpec<'static>> = Vec::new();
    for item in &file.items {
        if let syn::Item::Impl(item_impl) = item {
            if !has_attr(&item_impl.attrs, "capability") {
                continue;
            }
            let Ok(cap) = CapabilityImpl::new(item_impl.clone(), false, "", "") else {
                continue;
            };
            classes.push(build_class_spec(&cap, &builder)?);
        }
    }

    let mut structs: BTreeMap<FieldName<'static>, PyroSchema<'static>> = BTreeMap::new();
    for struct_name in builder.struct_names() {
        if let Some(schema) = builder.schema_for(struct_name)? {
            structs.insert(struct_name.to_string().into(), schema);
        }
    }

    Ok(InterfaceSpec {
        capability: name.to_string().into(),
        description: extract_doc_string(&file.attrs).map(Into::into),
        classes,
        structs,
    })
}

// =============================================================================
// Helpers
// =============================================================================

/// A method's `Ok` value as a single field named `output`, or nothing for `()`.
fn output_schema(ok_type: &Type, builder: &SchemaBuilder) -> syn::Result<PyroSchema<'static>> {
    if matches!(ok_type, Type::Tuple(t) if t.elems.is_empty()) {
        return Ok(PyroSchema::empty());
    }
    Ok(PyroSchema::new(vec![builder.field("output", ok_type, "")?]))
}

/// `#[derive(..., Layout, ...)]`: a struct that's on the wire without a
/// pyroduct attribute.
fn derives_layout(attrs: &[Attribute]) -> bool {
    attrs
        .iter()
        .filter(|a| a.path().is_ident("derive"))
        .any(|a| {
            let mut found = false;
            let _ = a.parse_nested_meta(|meta| {
                found |= meta
                    .path
                    .segments
                    .last()
                    .is_some_and(|s| s.ident == "Layout");
                Ok(())
            });
            found
        })
}

fn extract_doc_string(attrs: &[Attribute]) -> Option<String> {
    let mut lines = Vec::new();
    for attr in attrs {
        if attr.path().is_ident("doc")
            && let Meta::NameValue(nv) = &attr.meta
            && let Expr::Lit(expr_lit) = &nv.value
            && let Lit::Str(lit_str) = &expr_lit.lit
        {
            lines.push(lit_str.value().trim().to_string());
        }
    }
    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pyro_schema::{PrimitiveDataType, PyroType};
    use quote::quote;

    fn names(schema: &PyroSchema<'_>) -> Vec<(String, PyroType)> {
        schema
            .fields()
            .map(|f| (f.name().to_string(), f.datatype().clone()))
            .collect()
    }

    #[test]
    fn test_spec_generation_full() {
        let file: syn::File = syn::parse2(quote! {
            /// The Client State
            #[config]
            pub struct MyClient {
                /// The id
                pub id: u32,
                pub name: String,
            }

            #[config]
            pub struct InputStruct {
                pub foo: Bytes,
            }

            /// The Server Implementation
            #[capability]
            impl MyServer {
                type Client = MyClient;

                fn new() -> Result<Self, CapturedError> { Ok(Self) }
                fn reset(&mut self) -> Result<(), CapturedError> { Ok(()) }
                fn register(&self, c: &MyClient) -> Result<(), CapturedError> { Ok(()) }

                /// Calculates a value
                fn calculate(&self, c: &MyClient, input: f32) -> Result<f32, CapturedError> {
                    Ok(input * 2.0)
                }

                /// Processes the data
                fn process(&self, c: &MyClient, data: &[u8], tag: Option<u8>) -> Result<InputStruct, CapturedError> {
                    todo!()
                }

                fn clear(&self, c: &MyClient) -> Result<(), CapturedError> { Ok(()) }
            }
        })
        .unwrap();

        let spec = build_spec("MyServer", &file).unwrap();
        assert_eq!(spec.capability, "MyServer");
        assert!(spec.description.is_none());

        let u32_ = PyroType::PrimitiveScalar(PrimitiveDataType::U32);
        let f32_ = PyroType::PrimitiveScalar(PrimitiveDataType::F32);

        let class = &spec.classes[0];
        assert_eq!(class.name, "my_server");
        assert_eq!(
            class.description.as_deref(),
            Some("The Server Implementation")
        );
        assert!(class.config.is_none());

        let client = class.client.as_ref().unwrap();
        assert_eq!(client.doc(), "The Client State");
        assert_eq!(
            names(client),
            [("id".into(), u32_.clone()), ("name".into(), PyroType::Str)]
        );
        assert_eq!(client.field(0).doc(), "The id");

        let [calculate, process, clear] = &class.methods[..] else {
            panic!("expected three methods, got {:?}", class.methods);
        };
        assert_eq!(calculate.name, "calculate");
        assert_eq!(calculate.description.as_deref(), Some("Calculates a value"));
        assert_eq!(names(&calculate.input), [("input".into(), f32_.clone())]);
        assert_eq!(names(&calculate.output), [("output".into(), f32_)]);

        // Borrowed inputs are described by what they travel as.
        assert_eq!(
            names(&process.input),
            [
                ("data".into(), PyroType::Bytes),
                (
                    "tag".into(),
                    PyroType::OptionPrimitiveScalar(PrimitiveDataType::U8)
                ),
            ]
        );
        // A struct output keeps its field names.
        let output = process.output.field(0);
        assert_eq!(
            output.datatype(),
            &PyroType::Group(vec![PyroType::Bytes].into())
        );
        let child_names: Vec<_> = (0..output.child_names_len())
            .map(|i| output.child_name(i))
            .collect();
        assert_eq!(child_names, ["foo"]);

        assert_eq!(clear.input.num_fields(), 0);
        assert_eq!(clear.output.num_fields(), 0);

        assert_eq!(
            spec.structs
                .keys()
                .map(|k| k.to_string())
                .collect::<Vec<_>>(),
            ["InputStruct", "MyClient"]
        );
        assert_eq!(&spec.structs["MyClient"], client);
    }
}
