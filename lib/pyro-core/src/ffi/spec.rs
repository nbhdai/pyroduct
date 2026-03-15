//! Generates interface specifications using the Pyro type system.
//!
//! The spec JSON uses `PyroSchema`, `PyroField`, and `PyroType` from the
//! `spec` crate. Type resolution is delegated to [`SchemaBuilder`] which has
//! full knowledge of every struct in the source file, so nested user-defined
//! types expand into proper `Group(fields)`.

use std::borrow::Cow;

use spec::{CapabilityFunc, ClassSpec, InterfaceSpec, PyroField, PyroSchema, PyroType};
use syn::{Attribute, Expr, Lit, Meta};

use crate::ffi::has_attr;

use super::capability::CapabilityImpl;
use crate::struct_doc::SchemaBuilder;

// =============================================================================
// SpecBuilder
// =============================================================================

pub fn build_class_spec<'b>(
    cap: &CapabilityImpl,
    builder: &'b SchemaBuilder,
) -> ClassSpec<'static> {
    let capability = cap.ident.state_tn.to_string();
    let description = extract_doc_string(&cap.attrs);

    let methods = cap
        .methods
        .iter()
        .map(|m| {
            let name = m.name.to_string().into();
            let description = extract_doc_string(&m.attrs).map(|s| s.into());
            let output = fn_output_to_pyro_type(&m.output, builder);

            let fields: Vec<PyroField<'static>> = m
                .inputs
                .iter()
                .map(|(ident, ty)| {
                    let data_type = builder.resolve_type(ty);
                    let nullable = SchemaBuilder::is_option(ty);
                    PyroField::new(Cow::Owned(ident.to_string()), data_type, nullable)
                })
                .collect();

            let input = PyroSchema {
                documentation: None,
                fields: fields.into(),
            };

            CapabilityFunc {
                name,
                description,
                input,
                output,
            }
        })
        .collect();

    let client = builder.schema_for(&cap.ident.client_tn.to_string());
    let config = if let Some(config_tn) = &cap.ident.config_tn {
        builder.schema_for(&config_tn.to_string())
    } else {
        None
    };

    ClassSpec {
        name: capability.into(),
        description: description.map(|s| s.into()),
        client,
        config,
        methods,
    }
}

pub fn build_spec(file: &syn::File) -> InterfaceSpec<'static> {
    // Pass 1: collect all MagmaDocumentation from structs in the file.
    let builder = SchemaBuilder::from_file(file);

    let mut classes: Vec<ClassSpec<'static>> = Vec::new();
    let mut covered_items = Vec::new();

    for item in &file.items {
        if let syn::Item::Impl(item_impl) = item {
            if !has_attr(&item_impl.attrs, "capability") {
                continue;
            }
            let Ok(cap) = CapabilityImpl::new(item_impl.clone(), false, "", "") else {
                continue;
            };
            classes.push(build_class_spec(&cap, &builder));

            covered_items.push(cap.ident.client_tn.to_string());
            if let Some(config_tn) = &cap.ident.config_tn {
                covered_items.push(config_tn.to_string())
            }
        }
    }

    let (capability, description) = classes
        .first()
        .map(|c| (c.name.clone(), c.description.clone()))
        .unwrap_or_else(|| (Cow::Borrowed(""), None));

    InterfaceSpec {
        capability,
        description,
        classes,
    }
}

// =============================================================================
// Helpers
// =============================================================================

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

fn fn_output_to_pyro_type(
    output: &super::paths::FnOutput,
    builder: &SchemaBuilder,
) -> PyroType<'static> {
    match output {
        super::paths::FnOutput::None => PyroType::Null,
        super::paths::FnOutput::Single(ty) => builder.resolve_type(ty),
        super::paths::FnOutput::Result(ok_ty, _err_ty) => builder.resolve_type(ok_ty),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

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

            fn new() -> Self { Self }
            fn reset(&mut self) {}
            fn register(&self, c: &MyClient) {}

            /// Calculates a value
            fn calculate(&self, c: &MyClient, input: f32) -> f32 {
                input * 2.0
            }

            /// Processes the data
            fn process(&self, c: &MyClient, data: Option<Vec<u8>>) -> Result<InputStruct, MyError> {
                todo!()
            }
        }
    }).unwrap();

        let output = build_spec(&file);

        let expected: InterfaceSpec<'static> = serde_json::from_str(
            r#"{
            "capability": "MyServer",
            "description": "The Server Implementation",
            "classes": [
                {
                    "name": "MyServer",
                    "description": "The Server Implementation",
                    "client": {
                        "name": "MyClient",
                        "documentation": "The Client State",
                        "fields": [
                            {
                                "name": "id",
                                "documentation": "The id",
                                "data_type": { "PrimitiveScalar": "U32" },
                                "nullable": false
                            },
                            {
                                "name": "name",
                                "documentation": null,
                                "data_type": "Str",
                                "nullable": false
                            }
                        ]
                    },
                    "config": null,
                    "methods": [
                        {
                            "name": "calculate",
                            "description": "Calculates a value",
                            "input": {
                                "documentation": null,
                                "fields": [
                                    {
                                        "name": "input",
                                        "documentation": null,
                                        "data_type": { "PrimitiveScalar": "F32" },
                                        "nullable": false
                                    }
                                ]
                            },
                            "output": { "PrimitiveScalar": "F32" }
                        },
                        {
                            "name": "process",
                            "description": "Processes the data",
                            "input": {
                                "documentation": null,
                                "fields": [
                                    {
                                        "name": "data",
                                        "documentation": null,
                                        "data_type": { "PrimitiveList": "U8" },
                                        "nullable": true
                                    }
                                ]
                            },
                            "output": {
                                "Group": [
                                    {
                                        "name": "foo",
                                        "documentation": null,
                                        "data_type": { "PrimitiveList": "U8" },
                                        "nullable": false
                                    }
                                ]
                            }
                        }
                    ]
                }
            ]
        }"#,
        )
        .unwrap();

        assert_eq!(&output, &expected);
    }
}
