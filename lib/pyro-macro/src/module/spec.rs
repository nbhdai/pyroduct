//! Generates module interface specifications using the Pyro type system.
//!
//! Parses a module source file, locates the `#[module(...)]` function, and
//! builds a [`ModuleFunc`] describing its input parameters and output schema.
//! The result is serialised to JSON and written to `module.json` alongside the
//! compiled artefact.
//!
//! Input schema
//! ------------
//! Every typed parameter of the annotated function becomes a [`PyroField`] in
//! the input [`PyroSchema`].  Type resolution is delegated to
//! [`SchemaBuilder`], so struct parameters expand into `Group(fields)`.
//!
//! Output schema
//! -------------
//! The output schema is derived from the `output = …` argument:
//!
//! | `output =`         | generated `__Output` struct                        |
//! |--------------------|---------------------------------------------------|
//! | `field`            | `{ field: <ReturnType> }`                          |
//! | `(f1, f2, …)`      | `{ f1: T1, f2: T2, … }` (from tuple return type)  |
//! | `StructName`       | fields of the named struct looked up in the file   |

use std::borrow::Cow;

use pyro_spec::{ModuleFunc, PyroField, PyroSchema};
use syn::{Attribute, Expr, FnArg, ItemFn, Lit, Meta, Pat, ReturnType, Type};

use crate::struct_doc::SchemaBuilder;

use super::parse::{ModuleAttrs, OutputSpec};

// =============================================================================
// Public entry point
// =============================================================================

/// Parse `content` (a module source file), locate the `#[module(...)]`
/// function, and return a pretty-printed JSON string describing it.
///
/// Returns `None` when no `#[module(...)]` function is found.
pub fn generate_module_spec(content: &str) -> syn::Result<Option<ModuleFunc<'static>>> {
    let file = syn::parse_file(content)?;
    let builder = SchemaBuilder::from_file(&file);

    for item in &file.items {
        if let syn::Item::Fn(item_fn) = item {
            if !super::has_module_attr(&item_fn.attrs) {
                continue;
            }

            let attr_tokens = super::extract_module_attr(&item_fn.attrs)?.ok_or_else(|| {
                syn::Error::new_spanned(
                    item_fn,
                    "Module attribute requires arguments: #[module(output = ...)]",
                )
            })?;

            let attrs: ModuleAttrs = syn::parse2(attr_tokens)?;
            let spec = ModuleSpecBuilder::build(item_fn, &attrs, &builder)?;

            return Ok(Some(spec.into()));
        }
    }

    Ok(None)
}

// =============================================================================
// Builder
// =============================================================================

pub struct ModuleSpecBuilder;

impl ModuleSpecBuilder {
    /// Build a [`ModuleFuncSpec`] from a parsed function and its `#[module(...)]` attrs.
    pub fn build(
        item_fn: &ItemFn,
        attrs: &ModuleAttrs,
        builder: &SchemaBuilder,
    ) -> syn::Result<ModuleFunc<'static>> {
        let name = item_fn.sig.ident.to_string();
        let description = extract_doc_string(&item_fn.attrs);

        // ── Input schema ─────────────────────────────────────────────────────
        let input_fields: Vec<PyroField<'static>> = item_fn
            .sig
            .inputs
            .iter()
            .filter_map(|arg| {
                if let FnArg::Typed(pat_type) = arg {
                    if let Pat::Ident(pat_ident) = &*pat_type.pat {
                        let field_name = pat_ident.ident.to_string();
                        let ty = &*pat_type.ty;
                        let data_type = builder.resolve_type(ty);
                        let nullable = SchemaBuilder::is_option(ty);
                        let doc = extract_doc_string(&pat_type.attrs);
                        let mut field = PyroField::new(Cow::Owned(field_name), data_type, nullable);
                        if let Some(d) = doc {
                            field = field.add_docstring(Cow::Owned(d));
                        }
                        return Some(field);
                    }
                }
                None
            })
            .collect();

        let input = PyroSchema::new(input_fields);

        // ── Output schema ────────────────────────────────────────────────────
        let ok_type = extract_result_ok_type(&item_fn.sig.output)?;
        let output = build_output_schema(&attrs.output, &ok_type, builder)?;

        let func = ModuleFunc {
            name: Cow::Owned(name),
            description: description.map(Cow::Owned),
            input,
            output,
        };

        Ok(func)
    }
}

// =============================================================================
// Helpers
// =============================================================================

/// Build the output [`PyroSchema`] from the `output = …` spec and the
/// function's `Ok` return type.
fn build_output_schema(
    spec: &OutputSpec,
    ok_type: &Type,
    builder: &SchemaBuilder,
) -> syn::Result<PyroSchema<'static>> {
    match spec {
        // output = single_field  →  { single_field: <ok_type> }
        OutputSpec::SingleField(field_name) => {
            let data_type = builder.resolve_type(ok_type);
            let nullable = SchemaBuilder::is_option(ok_type);
            let field = PyroField::new(Cow::Owned(field_name.to_string()), data_type, nullable);
            Ok(PyroSchema::new(vec![field]))
        }

        // output = (f1, f2, …)  →  one field per tuple element
        OutputSpec::TupleFields(field_names) => {
            let tuple_types = extract_tuple_types(ok_type)?;

            if tuple_types.len() != field_names.len() {
                return Err(syn::Error::new_spanned(
                    ok_type,
                    format!(
                        "output field count ({}) does not match tuple element count ({})",
                        field_names.len(),
                        tuple_types.len()
                    ),
                ));
            }

            let fields: Vec<PyroField<'static>> = field_names
                .iter()
                .zip(tuple_types.iter())
                .map(|(name, ty)| {
                    let data_type = builder.resolve_type(ty);
                    let nullable = SchemaBuilder::is_option(ty);
                    PyroField::new(Cow::Owned(name.to_string()), data_type, nullable)
                })
                .collect();

            Ok(PyroSchema::new(fields))
        }

        // output = StructName  →  look up struct in the file registry
        OutputSpec::Struct => {
            // The return type must be a simple path — use it to look up the
            // schema from the builder registry.
            let schema = match ok_type {
                Type::Path(type_path) => {
                    if let Some(seg) = type_path.path.segments.last() {
                        builder.schema_for(&seg.ident.to_string())
                    } else {
                        None
                    }
                }
                _ => None,
            };

            Ok(schema.map(|s| s.into_owned()).unwrap_or_else(|| {
                // Fallback: resolve as a single anonymous field
                let data_type = builder.resolve_type(ok_type);
                let nullable = SchemaBuilder::is_option(ok_type);
                PyroSchema::new(vec![PyroField::new(
                    Cow::Borrowed("output"),
                    data_type,
                    nullable,
                )])
            }))
        }
    }
}

/// Extract the `Ok` type from `Result<T, _>` or `Result<T>`.
fn extract_result_ok_type(ret: &ReturnType) -> syn::Result<&Type> {
    match ret {
        ReturnType::Default => Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "module function must return Result<T>",
        )),
        ReturnType::Type(_, ty) => {
            if let Type::Path(type_path) = &**ty {
                if let Some(seg) = type_path.path.segments.last() {
                    if seg.ident == "Result" {
                        if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                            if let Some(syn::GenericArgument::Type(ok_ty)) = args.args.first() {
                                return Ok(ok_ty);
                            }
                        }
                    }
                }
            }
            Err(syn::Error::new_spanned(
                &**ty,
                "module function must return Result<T>",
            ))
        }
    }
}

/// Extract element types from a tuple type `(T1, T2, …)`.
fn extract_tuple_types(ty: &Type) -> syn::Result<Vec<&Type>> {
    if let Type::Tuple(tuple) = ty {
        Ok(tuple.elems.iter().collect())
    } else {
        Err(syn::Error::new_spanned(
            ty,
            "expected tuple return type for multi-field output",
        ))
    }
}

/// Collect `/// doc` comments from a slice of attributes into a single string.
fn extract_doc_string(attrs: &[Attribute]) -> Option<String> {
    let lines: Vec<String> = attrs
        .iter()
        .filter_map(|attr| {
            if !attr.path().is_ident("doc") {
                return None;
            }
            if let Meta::NameValue(nv) = &attr.meta {
                if let Expr::Lit(expr_lit) = &nv.value {
                    if let Lit::Str(s) = &expr_lit.lit {
                        return Some(s.value().trim().to_string());
                    }
                }
            }
            None
        })
        .collect();

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

    // ── Single field output ──────────────────────────────────────────────────

    #[test]
    fn test_single_field_output() {
        let src = r#"
            #[module(output = message)]
            fn call(input: &str) -> Result<String> {
                Ok(format!("hello {}", input))
            }
        "#;

        let v = generate_module_spec(src).unwrap().unwrap();

        assert_eq!(v.name, "call");
        assert!(v.description.is_none());

        // input: one field called `input` of type Str
        let in_fields = &v.input.fields;
        assert_eq!(in_fields[0].name, "input");

        // output: one field called `message` of type Str
        let out_fields = &v.output.fields;
        assert_eq!(out_fields[0].name, "message");
    }

    // ── Tuple field output ───────────────────────────────────────────────────

    #[test]
    fn test_tuple_output() {
        let src = r#"
            #[module(output = (score, label))]
            fn classify(text: String) -> Result<(f32, String)> {
                Ok((0.9, "positive".into()))
            }
        "#;

        let v = generate_module_spec(src).unwrap().unwrap();

        let out_fields = &v.output.fields;
        assert_eq!(out_fields[0].name, "score");
        assert_eq!(out_fields[1].name, "label");
    }

    // ── Struct output ────────────────────────────────────────────────────────

    #[test]
    fn test_struct_output() {
        let src = r#"
            #[config]
            struct Output {
                embedding: Vec<f32>,
                tokens: u32,
            }

            /// Embed a piece of text.
            #[module(output = Output)]
            fn embed(text: String, model: String) -> Result<Output> {
                todo!()
            }
        "#;

        let v = generate_module_spec(src).unwrap().unwrap();

        assert_eq!(v.name, "embed");
        assert_eq!(v.description.unwrap(), "Embed a piece of text.");

        let in_fields = &v.input.fields;
        assert_eq!(in_fields.len(), 2);
        assert_eq!(in_fields[0].name, "text");
        assert_eq!(in_fields[1].name, "model");

        let out_fields = &v.output.fields;
        assert_eq!(out_fields[0].name, "embedding");
        assert_eq!(out_fields[1].name, "tokens");
    }

    // ── No module function ───────────────────────────────────────────────────

    #[test]
    fn test_no_module_function() {
        let src = r#"
            fn plain(x: u32) -> u32 { x }
        "#;
        let result = generate_module_spec(src).unwrap();
        assert!(result.is_none());
    }
}
