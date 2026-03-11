//! Generates interface specifications using the Pyro type system.
//!
//! The spec JSON uses `PyroSchema`, `PyroField`, and `PyroType` from the
//! `spec` crate. Type resolution is delegated to [`SchemaBuilder`] which has
//! full knowledge of every struct in the source file, so nested user-defined
//! types expand into proper `Group(fields)`.

use std::borrow::Cow;
use std::collections::HashMap;

use serde::Serialize;
use spec::{PyroField, PyroSchema, PyroType};
use syn::{Attribute, Expr, Lit, Meta};

use crate::format::documentation::MagmaDocumentation;

use super::capability::CapabilityImpl;
use super::config::CapConfig;
use crate::struct_doc::SchemaBuilder;

// =============================================================================
// Spec types — backed by PyroSchema / PyroField / PyroType
// =============================================================================

/// The root specification object.
#[derive(Serialize)]
pub struct InterfaceSpec {
    pub capability: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub client: Option<NamedSchema>,

    pub methods: Vec<MethodSpec>,

    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub items: HashMap<String, PyroSchema<'static>>,
}

/// A named schema — wraps a name with a `PyroSchema`.
#[derive(Serialize)]
pub struct NamedSchema {
    pub name: String,
    #[serde(flatten)]
    pub schema: PyroSchema<'static>,
}

#[derive(Serialize)]
pub struct MethodSpec {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub parameters: PyroSchema<'static>,
    pub return_type: PyroType<'static>,
}

/// The root specification object for a Config.
#[derive(Serialize)]
pub struct ConfigSpec {
    pub name: String,
    #[serde(flatten)]
    pub schema: PyroSchema<'static>,
}

// =============================================================================
// SpecBuilder
// =============================================================================

pub struct SpecBuilder<'b> {
    spec: InterfaceSpec,
    target_client_name: String,
    builder: &'b SchemaBuilder,
}

impl<'b> SpecBuilder<'b> {
    pub fn new(cap: &CapabilityImpl, builder: &'b SchemaBuilder) -> Self {
        let capability = cap.ident.state_tn.to_string();
        let target_client_name = cap.ident.client_tn.to_string();
        let description = extract_doc_string(&cap.attrs);

        let methods = cap
            .methods
            .iter()
            .map(|m| {
                let name = m.name.to_string();
                let description = extract_doc_string(&m.attrs);
                let return_type = fn_output_to_pyro_type(&m.output, builder);

                let param_fields: Vec<PyroField<'static>> = m
                    .inputs
                    .iter()
                    .map(|(ident, ty)| {
                        let data_type = builder.resolve_type(ty);
                        let nullable = SchemaBuilder::is_option(ty);
                        PyroField::new(Cow::Owned(ident.to_string()), data_type, nullable)
                    })
                    .collect();

                MethodSpec {
                    name,
                    description,
                    parameters: PyroSchema::new(param_fields),
                    return_type,
                }
            })
            .collect();

        Self {
            spec: InterfaceSpec {
                capability,
                description,
                client: None,
                methods,
                items: HashMap::new(),
            },
            target_client_name,
            builder,
        }
    }

    pub fn append(&mut self, doc: &MagmaDocumentation) {
        let name = doc.ident.to_string();
        let schema = self
            .builder
            .schema_for(&name)
            .unwrap_or_else(|| PyroSchema::empty());

        if name == self.target_client_name {
            self.spec.client = Some(NamedSchema { name, schema });
        } else {
            self.spec.items.insert(name, schema);
        }
    }

    pub fn build(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.spec)
    }
}

// =============================================================================
// ConfigSpecBuilder
// =============================================================================

pub struct ConfigSpecBuilder;

impl ConfigSpecBuilder {
    pub fn build(config: &CapConfig, builder: &SchemaBuilder) -> Result<String, serde_json::Error> {
        let name = config.input.ident.to_string();
        let schema = builder.schema_for(&name).unwrap_or_else(|| {
            // Fallback: build from the struct fields directly
            let description = extract_doc_string(&config.input.attrs);
            let fields = parse_struct_fields_to_pyro(&config.input.fields, builder);
            let mut s = PyroSchema::new(fields);
            if let Some(d) = &description {
                s = s.add_docstring(Cow::Owned(d.clone()));
            }
            s
        });

        let spec = ConfigSpec { name, schema };
        serde_json::to_string_pretty(&spec)
    }
}

// =============================================================================
// Helpers
// =============================================================================

fn parse_struct_fields_to_pyro(
    fields: &syn::Fields,
    builder: &SchemaBuilder,
) -> Vec<PyroField<'static>> {
    let mut result = Vec::new();
    if let syn::Fields::Named(named) = fields {
        for f in &named.named {
            if let Some(ident) = &f.ident {
                let data_type = builder.resolve_type(&f.ty);
                let nullable = SchemaBuilder::is_option(&f.ty);
                let doc = extract_doc_string(&f.attrs);
                let mut field = PyroField::new(Cow::Owned(ident.to_string()), data_type, nullable);
                if let Some(d) = doc {
                    field = field.add_docstring(Cow::Owned(d));
                }
                result.push(field);
            }
        }
    }
    result
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

#[cfg(test)]
mod tests {
    use crate::format::bridgeable::DocRec;

    use super::*;
    use quote::quote;
    use serde_json::Value;
    use syn::parse2;

    use super::super::capability::CapabilityImpl;

    fn assert_json_eq(actual_str: &str, expected_str: &str) {
        let actual: Value = serde_json::from_str(actual_str).expect("Generated JSON was invalid");
        let expected: Value =
            serde_json::from_str(expected_str).expect("Expected JSON string was invalid");

        if actual != expected {
            println!(
                "EXPECTED:\n{}",
                serde_json::to_string_pretty(&expected).unwrap()
            );
            println!(
                "ACTUAL:\n{}",
                serde_json::to_string_pretty(&actual).unwrap()
            );
            panic!("JSON mismatch");
        }
    }

    /// Helper: build a SchemaBuilder from token streams by assembling a file.
    fn schema_builder_from(structs: &[proc_macro2::TokenStream]) -> SchemaBuilder {
        let combined = quote! { #(#structs)* };
        let file: syn::File = syn::parse2(combined).unwrap();
        SchemaBuilder::from_file(&file)
    }

    #[test]
    fn test_spec_generation_full() {
        let client_tokens = quote! {
            /// The Client State
            #[interface]
            pub struct MyClient {
                /// The id
                pub id: u32,
                pub name: String,
            }
        };

        let other_tokens = quote! {
            #[interface]
            pub struct InputStruct {
                pub foo: Bytes,
            }
        };

        let impl_tokens = quote! {
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
                    Ok(0)
                }
            }
        };

        let builder = schema_builder_from(&[client_tokens.clone(), other_tokens.clone()]);

        let cap_impl =
            CapabilityImpl::new(parse2(impl_tokens).unwrap(), true, "cap_name", "0.1.0").unwrap();
        let client_item =
            MagmaDocumentation::from_item(&parse2(client_tokens).unwrap(), DocRec::NoReq).unwrap();
        let input_item =
            MagmaDocumentation::from_item(&parse2(other_tokens).unwrap(), DocRec::NoReq).unwrap();
        let mut spec_builder = SpecBuilder::new(&cap_impl, &builder);
        spec_builder.append(&client_item);
        spec_builder.append(&input_item);
        let output = spec_builder.build().unwrap();

        let expected = serde_json::json!({
            "capability": "MyServer",
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
            "methods": [
                {
                    "name": "calculate",
                    "description": "Calculates a value",
                    "parameters": {
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
                    "return_type": { "PrimitiveScalar": "F32" }
                },
                {
                    "name": "process",
                    "description": "Processes the data",
                    "parameters": {
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
                    "return_type": {
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
            ],
            "items": {
                "InputStruct": {
                    "documentation": null,
                    "fields": [
                        {
                            "name": "foo",
                            "documentation": null,
                            "data_type": { "PrimitiveList": "U8" },
                            "nullable": false
                        }
                    ]
                }
            }
        });

        assert_json_eq(&output, &serde_json::to_string_pretty(&expected).unwrap());
    }

    #[test]
    fn test_nested_struct_in_return_type() {
        let structs = quote! {
            struct Inner {
                value: i64,
            }
            struct Outer {
                inner: Inner,
                count: u32,
            }
        };

        let impl_tokens = quote! {
            /// A capability
            #[capability]
            impl MySvc {
                type Client = Outer;

                fn new() -> Self { Self }
                fn reset(&mut self) {}
                fn register(&self, c: &Outer) {}

                /// Gets inner
                fn get_inner(&self, c: &Outer) -> Inner {
                    todo!()
                }
            }
        };

        let file: syn::File = syn::parse2(quote! { #structs #impl_tokens }).unwrap();
        let builder = SchemaBuilder::from_file(&file);

        let cap_impl =
            CapabilityImpl::new(parse2(impl_tokens).unwrap(), true, "cap_name", "0.1.0").unwrap();
        let outer_doc = MagmaDocumentation::from_item(
            &parse2(quote! {
                struct Outer { inner: Inner, count: u32 }
            })
            .unwrap(),
            DocRec::NoReq,
        )
        .unwrap();

        let mut spec_builder = SpecBuilder::new(&cap_impl, &builder);
        spec_builder.append(&outer_doc);
        let output = spec_builder.build().unwrap();
        let parsed: Value = serde_json::from_str(&output).unwrap();

        // Return type should be a resolved Group with Inner's field
        let ret = &parsed["methods"][0]["return_type"];
        assert!(ret["Group"].is_array());
        assert_eq!(ret["Group"][0]["name"], "value");
        assert_eq!(ret["Group"][0]["data_type"]["PrimitiveScalar"], "I64");
    }
}
