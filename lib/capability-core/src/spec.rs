//! Generates OpenAPI-style interface specifications
use std::collections::HashMap;

use quote::quote;
use serde::Serialize;
use syn::{Attribute, Expr, Lit, Meta};

use crate::capability::CapabilityImpl;
use crate::client::CapInterfaceItem;

/// The root specification object
#[derive(Serialize)]
pub struct InterfaceSpec {
    /// The name of the capability (Server struct)
    pub capability: String,
    
    /// The main Client struct specification
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client: Option<ClientSpec>,
    
    /// Description of the capability (doc comments)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    /// List of RPC methods
    pub methods: Vec<MethodSpec>,
    
    /// Auxiliary structs used in the interface
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub items: HashMap<String, ItemSpec>,
}

#[derive(Serialize)]
pub struct ClientSpec {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub fields: HashMap<String, FieldDef>,
}

#[derive(Serialize)]
pub struct ItemSpec {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub fields: HashMap<String, FieldDef>,
}

#[derive(Serialize)]
pub struct MethodSpec {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub parameters: Vec<ParamSpec>,
    pub return_type: String,
}

#[derive(Serialize)]
pub struct ParamSpec {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub r#type: String,
}

/// Handles polymorphic field definitions:
/// - "name": "String" (Simple)
/// - "id": { "type": "u32", "description": "..." } (Detailed)
#[derive(Serialize)]
#[serde(untagged)]
pub enum FieldDef {
    Simple(String),
    Detailed {
        #[serde(rename = "type")]
        r#type: String,
        description: String,
    },
}

pub struct SpecBuilder {
    spec: InterfaceSpec,
    /// Name of the struct defined as `type Client = ...` in the capability
    target_client_name: String,
}

impl SpecBuilder {
    pub fn new(cap: &CapabilityImpl) -> Self {
        let capability = cap.ident.state_tn.to_string();
        let target_client_name = cap.ident.client_tn.to_string();
        let description = extract_doc_string(&cap.attrs);

        let methods = cap
            .methods
            .iter()
            .map(|m| {
                let name = m.name.to_string();
                let description = extract_doc_string(&m.attrs);
                let return_type = clean_return_type(&m.output);

                let parameters = m
                    .inputs
                    .iter()
                    .map(|(ident, ty)| ParamSpec {
                        name: ident.to_string(),
                        r#type: clean_type(ty),
                        description: None, // Method params usually don't have individual docs in syn parsing easily available without extra parsing
                    })
                    .collect();

                MethodSpec {
                    name,
                    description,
                    parameters,
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
        }
    }

    pub fn append(&mut self, client: &CapInterfaceItem) {
        let name = client.input.ident.to_string();
        let description = extract_doc_string(&client.input.attrs);
        let fields = parse_struct_fields(&client.input.fields);

        if name == self.target_client_name {
            self.spec.client = Some(ClientSpec {
                name,
                description,
                fields,
            });
        } else {
            self.spec.items.insert(name, ItemSpec {
                description,
                fields,
            });
        }
    }

    pub fn build(&self) -> anyhow::Result<String> {
        serde_json::to_string_pretty(&self.spec).map_err(|e| anyhow::anyhow!(e))
    }
}

// --- Helpers ---

fn parse_struct_fields(fields: &syn::Fields) -> HashMap<String, FieldDef> {
    let mut map = HashMap::new();
    if let syn::Fields::Named(named) = fields {
        for f in &named.named {
            if let Some(ident) = &f.ident {
                let type_name = clean_type(&f.ty);
                let doc = extract_doc_string(&f.attrs);
                let def = match doc {
                    Some(d) => FieldDef::Detailed {
                        r#type: type_name,
                        description: d,
                    },
                    None => FieldDef::Simple(type_name),
                };
                map.insert(ident.to_string(), def);
            }
        }
    }
    map
}

/// Helper to format types closer to the user expectation (Option<Vec<u8>>)
/// rather than the default quote! spacing (Option < Vec < u8 > >)
fn clean_type(ty: &syn::Type) -> String {
    let s = quote!(#ty).to_string();
    s.replace(" < ", "<")
     .replace(" > ", ">")
     .replace(" >", ">")     // Clean trailing brackets
     .replace(" , ", ", ")   // Normalize comma spacing
}

fn clean_return_type(ret: &syn::ReturnType) -> String {
    match ret {
        syn::ReturnType::Default => "()".to_string(),
        syn::ReturnType::Type(_, ty) => clean_type(ty),
    }
}

fn extract_doc_string(attrs: &[Attribute]) -> Option<String> {
    let mut lines = Vec::new();
    for attr in attrs {
        if attr.path().is_ident("doc") {
            if let Meta::NameValue(nv) = &attr.meta {
                if let Expr::Lit(expr_lit) = &nv.value {
                    if let Lit::Str(lit_str) = &expr_lit.lit {
                        let val = lit_str.value();
                        lines.push(val.trim().to_string());
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
    use super::*;
    use quote::quote;
    use serde_json::Value;
    use syn::parse2;

    /// Helper to normalize JSON for comparison
    fn assert_json_eq(actual_str: &str, expected_str: &str) {
        let actual: Value = serde_json::from_str(actual_str)
            .expect("Generated JSON was invalid");
        let expected: Value = serde_json::from_str(expected_str)
            .expect("Expected JSON string was invalid");

        if actual != expected {
            println!("EXPECTED:\n{}", serde_json::to_string_pretty(&expected).unwrap());
            println!("ACTUAL:\n{}", serde_json::to_string_pretty(&actual).unwrap());
            panic!("JSON mismatch");
        }
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
                fn new_client(&self, c: &MyClient) {}

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

        let cap_impl = CapabilityImpl::new(parse2(impl_tokens).unwrap(), true).unwrap();
        let client_item = CapInterfaceItem::new(parse2(client_tokens).unwrap(), false).unwrap();
        let input_item = CapInterfaceItem::new(parse2(other_tokens).unwrap(), false).unwrap();
        let mut spec_builder = SpecBuilder::new(&cap_impl);
        spec_builder.append(&client_item);
        spec_builder.append(&input_item);
        let output = spec_builder.build().unwrap();

        let expected = r#"{
          "capability": "MyServer",
          "description": "The Server Implementation",
          "client": {
                "name": "MyClient",
                "description": "The Client State",
                "fields": {
                    "id": { "type": "u32", "description": "The id" },
                    "name": "String"
                }
          },
          "methods": [
            {
              "name": "calculate",
              "description": "Calculates a value",
              "parameters": [
                { "name": "input", "type": "f32" }
              ],
              "return_type": "f32"
            },
            {
              "name": "process",
              "description": "Processes the data",
              "parameters": [
                { "name": "data", "type": "Option<Vec<u8>>" }
              ],
              "return_type": "Result<InputStruct, MyError>"
            }
          ],
          "items": {
            "InputStruct": {
                "fields": {
                    "foo": "Bytes"
                }
            }
          }
        }"#;

        assert_json_eq(&output, expected);
    }
}