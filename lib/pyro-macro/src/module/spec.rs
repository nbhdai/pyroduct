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
//! [`SchemaBuilder`], so struct parameters expand into `Group`s, and a
//! borrowed parameter is described by the owned type it travels as.
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

use pyro_schema::interface::{InterfaceSpec, ModuleFunc, ModuleKind};
use pyro_schema::{PyroField, PyroSchema};
use pyroformat_macro::schema::SchemaBuilder;
use syn::{Attribute, Expr, FnArg, ItemFn, Lit, Meta, Pat, ReturnType, Type};

use crate::ffi::spec::schema_builder;

use super::parse::{ModuleAttrs, OutputSpec};

// =============================================================================
// Public entry point
// =============================================================================

/// Parse `content` (a module source file), locate the `#[module(...)]`
/// function, and return a pretty-printed JSON string describing it.
///
/// Returns `None` when no `#[module(...)]` function is found.
pub fn generate_module_spec(
    content: &str,
    dep_interfaces: &[InterfaceSpec<'static>],
) -> syn::Result<Option<ModuleFunc<'static>>> {
    let file = syn::parse_file(content)?;
    let builder = schema_builder(&file, dep_interfaces)?;

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

            return Ok(Some(spec));
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
                if let FnArg::Typed(pat_type) = arg
                    && let Pat::Ident(pat_ident) = &*pat_type.pat
                {
                    let doc = extract_doc_string(&pat_type.attrs).unwrap_or_default();
                    return Some(builder.field(&pat_ident.ident.to_string(), &pat_type.ty, &doc));
                }
                None
            })
            .collect::<syn::Result<_>>()?;

        let input = PyroSchema::new(input_fields);

        // ── Output schema ────────────────────────────────────────────────────
        let ok_type = extract_result_ok_type(&item_fn.sig.output)?;
        let ok_type = if attrs.session {
            if let Type::Path(inner_path) = ok_type
                && let Some(seg) = inner_path.path.segments.last()
                && seg.ident == "SessionResponse"
                && let syn::PathArguments::AngleBracketed(inner_args) = &seg.arguments
                && let Some(syn::GenericArgument::Type(output_ty)) = inner_args.args.first()
            {
                output_ty
            } else {
                ok_type
            }
        } else {
            ok_type
        };
        let output = build_output_schema(&attrs.output, ok_type, builder)?;

        let kind = if attrs.session {
            let num_inputs = item_fn.sig.inputs.len();
            if num_inputs == 2 {
                ModuleKind::Session
            } else if num_inputs == 3 {
                ModuleKind::SessionDiff
            } else {
                ModuleKind::Normal
            }
        } else {
            ModuleKind::Normal
        };

        let func = ModuleFunc {
            name: name.into(),
            description: description.map(Into::into),
            input,
            output,
            kind,
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
        OutputSpec::SingleField(field_name) => Ok(PyroSchema::new(vec![builder.field(
            &field_name.to_string(),
            ok_type,
            "",
        )?])),

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
                .map(|(name, ty)| builder.field(&name.to_string(), ty, ""))
                .collect::<syn::Result<_>>()?;

            Ok(PyroSchema::new(fields))
        }

        // output = StructName  →  look up struct in the file registry
        OutputSpec::Struct => {
            // The return type must be a simple path — use it to look up the
            // schema from the builder registry.
            let schema = match ok_type {
                Type::Path(type_path) => match type_path.path.segments.last() {
                    Some(seg) => builder.schema_for(&seg.ident.to_string())?,
                    None => None,
                },
                _ => None,
            };

            match schema {
                Some(schema) => Ok(schema),
                // Fallback: resolve as a single anonymous field
                None => Ok(PyroSchema::new(vec![builder.field("output", ok_type, "")?])),
            }
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
            if let Type::Path(type_path) = &**ty
                && let Some(seg) = type_path.path.segments.last()
                && seg.ident == "Result"
                && let syn::PathArguments::AngleBracketed(args) = &seg.arguments
                && let Some(syn::GenericArgument::Type(ok_ty)) = args.args.first()
            {
                return Ok(ok_ty);
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
            if let Meta::NameValue(nv) = &attr.meta
                && let Expr::Lit(expr_lit) = &nv.value
                && let Lit::Str(s) = &expr_lit.lit
            {
                return Some(s.value().trim().to_string());
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
    use pyro_schema::{PrimitiveDataType, PyroType};

    fn names(schema: &PyroSchema<'_>) -> Vec<String> {
        schema.fields().map(|f| f.name().to_string()).collect()
    }

    /// The names of the groups a field reaches, in pre-order.
    fn children(field: pyro_schema::ChildField<'_>) -> Vec<String> {
        (0..field.child_names_len())
            .map(|i| field.child_name(i).to_string())
            .collect()
    }

    // ── Single field output ──────────────────────────────────────────────────

    #[test]
    fn test_single_field_output() {
        let src = r#"
            #[module(output = message)]
            fn call(input: &str) -> Result<String> {
                Ok(format!("hello {}", input))
            }
        "#;

        let v = generate_module_spec(src, &[]).unwrap().unwrap();

        assert_eq!(v.name, "call");
        assert!(v.description.is_none());
        assert_eq!(v.kind, ModuleKind::Normal);

        // input: one field called `input`, a `&str` travelling as a `String`
        assert_eq!(names(&v.input), ["input"]);
        assert_eq!(v.input.field(0).datatype(), &PyroType::Str);

        // output: one field called `message` of type Str
        assert_eq!(names(&v.output), ["message"]);
        assert_eq!(v.output.field(0).datatype(), &PyroType::Str);
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

        let v = generate_module_spec(src, &[]).unwrap().unwrap();

        assert_eq!(names(&v.output), ["score", "label"]);
        assert_eq!(
            v.output.types(),
            [
                PyroType::PrimitiveScalar(PrimitiveDataType::F32),
                PyroType::Str
            ]
        );
    }

    // ── Struct output ────────────────────────────────────────────────────────

    #[test]
    fn test_struct_output() {
        let src = r#"
            #[config]
            struct Output {
                /// One per dimension
                embedding: Vec<f32>,
                tokens: u32,
            }

            /// Embed a piece of text.
            #[module(output = Output)]
            fn embed(text: String, model: String) -> Result<Output> {
                todo!()
            }
        "#;

        let v = generate_module_spec(src, &[]).unwrap().unwrap();

        assert_eq!(v.name, "embed");
        assert_eq!(v.description.unwrap(), "Embed a piece of text.");

        assert_eq!(names(&v.input), ["text", "model"]);

        assert_eq!(names(&v.output), ["embedding", "tokens"]);
        assert_eq!(v.output.field(0).doc(), "One per dimension");
        assert_eq!(
            v.output.field(0).datatype(),
            &PyroType::PrimitiveList(PrimitiveDataType::F32)
        );
    }

    // ── Session module with foreign struct output ────────────────────────────

    #[test]
    fn test_session_foreign_struct() {
        use pyro_schema::PyroField;
        use std::collections::BTreeMap;

        let src = r#"
            #[module(session, output = ChatMessage)]
            fn process(
                prior: Vec<ChatMessage>,
                input: ChatMessageRef<'_>,
            ) -> Result<SessionResponse<ChatMessage>> {
                todo!()
            }
        "#;

        // Create a mock dependency interface that declares ChatMessage struct
        let mut structs = BTreeMap::new();
        structs.insert(
            "ChatMessage".into(),
            PyroSchema::new(vec![
                PyroField::new("role", PyroType::Str),
                PyroField::new("content", PyroType::Str),
            ]),
        );

        let dep = InterfaceSpec {
            capability: "llm".into(),
            description: None,
            classes: vec![],
            structs,
        };

        let v = generate_module_spec(src, &[dep]).unwrap().unwrap();

        assert_eq!(v.name, "process");
        assert_eq!(v.kind, ModuleKind::Session);

        let message = PyroType::Group(vec![PyroType::Str, PyroType::Str].into());

        // prior: Vec<ChatMessage> is a list of the dependency's group
        assert_eq!(names(&v.input), ["prior", "input"]);
        assert_eq!(
            v.input.field(0).datatype(),
            &PyroType::List(pyro_schema::Bow::new(message.clone()))
        );
        assert_eq!(children(v.input.field(0)), ["role", "content"]);

        // input: ChatMessageRef is the view of ChatMessage, so the same group
        assert_eq!(v.input.field(1).datatype(), &message);
        assert_eq!(children(v.input.field(1)), ["role", "content"]);

        // ChatMessage isn't declared in this file, so the output falls back to
        // a single `output` field of the dependency's group.
        assert_eq!(names(&v.output), ["output"]);
        assert_eq!(v.output.field(0).datatype(), &message);
        assert_eq!(children(v.output.field(0)), ["role", "content"]);
    }

    // ── No module function ───────────────────────────────────────────────────

    #[test]
    fn test_no_module_function() {
        let src = r#"
            fn plain(x: u32) -> u32 { x }
        "#;
        let result = generate_module_spec(src, &[]).unwrap();
        assert!(result.is_none());
    }
}
