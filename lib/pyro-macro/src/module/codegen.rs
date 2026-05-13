use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{FnArg, Ident, ItemFn, Pat, Result, ReturnType, Type, parse_quote};

use super::parse::{ModuleAttrs, OutputSpec};

pub fn expand(attrs: ModuleAttrs, input_fn: ItemFn) -> Result<TokenStream> {
    let fn_name = &input_fn.sig.ident;
    let fn_vis = &input_fn.vis;
    let fn_block = &input_fn.block;
    let fn_attrs = &input_fn.attrs;

    // Extract parameters
    let params: Vec<_> = input_fn
        .sig
        .inputs
        .iter()
        .filter_map(|arg| {
            if let FnArg::Typed(pat_type) = arg {
                if let Pat::Ident(pat_ident) = &*pat_type.pat {
                    let name = pat_ident.ident.clone();
                    let ty = (*pat_type.ty).clone();
                    return Some((name, ty));
                }
            }
            None
        })
        .collect();

    // Validate session module requirements and extract types
    let return_type = extract_result_ok_type(&input_fn.sig.output)?;
    let output_type = if attrs.session {
        extract_session_inner_type(&input_fn.sig.output)?
    } else {
        return_type.clone()
    };

    // Generate __Output struct and mapping based on output spec
    let (output_struct, output_mapping, output_name) =
        generate_output(&attrs.output, &output_type, attrs.session)?;

    // Generate the call arguments (extract from input struct)
    let call_args: Vec<_> = params
        .iter()
        .map(|(name, ty)| {
            let name_str = name.to_string();
            quote! { input.get_value::<#ty>(#name_str).ok_or_else(|| ::pyroduct::CapturedError::new(format!("Missing {}", #name_str)))? }
        })
        .collect();

    // Generate the original function parameters
    let original_fn_params: Vec<_> = params
        .iter()
        .map(|(name, ty)| quote! { #name: #ty })
        .collect();

    let expanded = quote! {
        #[unsafe(no_mangle)]
        pub extern "C" fn call_extern(input_ptr: *mut u8) -> *const u8 {

            #output_struct

            let call = |input: ::pyroduct::PyroRow<'_>| {
                #fn_name(#(#call_args),*).map(|result| {
                    #output_mapping
                })
            };


            ::pyroduct::wasm::wasm_row_main::<#output_name, _>(input_ptr, call)
        }

        #(#fn_attrs)*
        #fn_vis fn #fn_name(#(#original_fn_params),*) -> ::pyroduct::wasm::ModuleResult<#return_type>
        #fn_block
    };

    Ok(expanded)
}

/// Extract the Ok type from Result<T, E>
fn extract_result_ok_type(ret: &ReturnType) -> Result<Type> {
    match ret {
        ReturnType::Default => Err(syn::Error::new(
            Span::call_site(),
            "Module function must return Result<T>",
        )),
        ReturnType::Type(_, ty) => {
            if let Type::Path(type_path) = &**ty {
                if let Some(segment) = type_path.path.segments.last() {
                    if segment.ident == "Result" {
                        if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                            if let Some(syn::GenericArgument::Type(ok_ty)) = args.args.first() {
                                return Ok(ok_ty.clone());
                            }
                        }
                    }
                }
            }
            Err(syn::Error::new(
                Span::call_site(),
                "Module function must return Result<T>",
            ))
        }
    }
}

/// Generate the __Output struct and the mapping expression
fn generate_output(
    spec: &OutputSpec,
    return_type: &Type,
    is_session: bool,
) -> Result<(TokenStream, TokenStream, Type)> {
    match spec {
        // Pattern 1: Single named field
        OutputSpec::SingleField(field_name) => {
            let struct_def = quote! {
                #[derive(::pyroduct::format::ToRow, ::pyroduct::format::Document)]
                struct __Output {
                    #field_name: #return_type,
                }
            };

            let mapping = if is_session {
                // Map SessionResponse<T> -> SessionResponse<__Output>
                quote! {
                    result.map(|inner| __Output {
                        #field_name: inner,
                    })
                }
            } else {
                quote! {
                    __Output {
                        #field_name: result,
                    }
                }
            };

            let output_name = if is_session {
                parse_quote!(::pyroduct::session::SessionResponse<__Output>)
            } else {
                parse_quote!(__Output)
            };

            Ok((struct_def, mapping, output_name))
        }

        // Pattern 2: Tuple with named fields
        OutputSpec::TupleFields(field_names) => {
            // Extract tuple element types from return_type
            let tuple_types = extract_tuple_types(return_type)?;

            if tuple_types.len() != field_names.len() {
                return Err(syn::Error::new(
                    Span::call_site(),
                    format!(
                        "Output field count ({}) doesn't match tuple element count ({})",
                        field_names.len(),
                        tuple_types.len()
                    ),
                ));
            }

            let field_defs: Vec<_> = field_names
                .iter()
                .zip(tuple_types.iter())
                .map(|(name, ty)| quote! { #name: #ty })
                .collect();

            let struct_def = quote! {
                #[derive(::pyroduct::format::ToRow, ::pyroduct::format::Document)]
                struct __Output {
                    #(#field_defs,)*
                }
            };

            if is_session {
                let field_mappings_inner: Vec<_> = field_names
                    .iter()
                    .enumerate()
                    .map(|(i, name)| {
                        let idx = syn::Index::from(i);
                        quote! { inner.#idx }
                    })
                    .collect();

                let mapping = quote! {
                    result.map(|inner| __Output {
                        #(#field_mappings_inner,)*
                    })
                };

                let output_name = parse_quote!(::pyroduct::session::SessionResponse<__Output>);
                Ok((struct_def, mapping, output_name))
            } else {
                let field_mappings: Vec<_> = field_names
                    .iter()
                    .enumerate()
                    .map(|(i, name)| {
                        let idx = syn::Index::from(i);
                        quote! { #name: result.#idx }
                    })
                    .collect();

                let mapping = quote! {
                    __Output {
                        #(#field_mappings,)*
                    }
                };

                Ok((struct_def, mapping, parse_quote!(__Output)))
            }
        }

        // Pattern 3: Existing struct that implements ToRow
        OutputSpec::Struct => {
            // No __Output struct needed, use the return type directly
            let struct_def = quote! {};

            let mapping = if is_session {
                // Pass through SessionResponse<Struct> as-is
                quote! { result }
            } else {
                quote! { result }
            };

            let output_name = if is_session {
                parse_quote!(::pyroduct::session::SessionResponse<#return_type>)
            } else {
                return_type.clone()
            };

            Ok((struct_def, mapping, output_name))
        }
    }
}

/// Extract element types from a tuple type
fn extract_tuple_types(ty: &Type) -> Result<Vec<&Type>> {
    if let Type::Tuple(tuple) = ty {
        Ok(tuple.elems.iter().collect())
    } else {
        Err(syn::Error::new(
            Span::call_site(),
            "Expected tuple return type for multi-field output",
        ))
    }
}

/// Validate that session modules have the required prior_input and prior_output parameters
fn validate_session_params(params: &[(Ident, Type)]) -> Result<()> {
    let mut has_prior_input = false;
    let mut has_prior_output = false;

    for (name, ty) in params {
        match name.to_string().as_str() {
            "prior_input" => has_prior_input = true,
            "prior_output" => {
                has_prior_output = true;
                // Validate that prior_output is a Vec
                if !is_vec_type(ty) {
                    return Err(syn::Error::new(
                        Span::call_site(),
                        "Session module's prior_output must be a Vec",
                    ));
                }
            }
            _ => {}
        }
    }

    if !has_prior_input || !has_prior_output {
        return Err(syn::Error::new(
            Span::call_site(),
            "Session modules require a \"prior_input\", \"prior_output\", and \"input\", not found",
        ));
    }

    Ok(())
}

/// Check if a type is a Vec<T>
fn is_vec_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Vec" {
                return true;
            }
        }
    }
    false
}

/// Extract the inner type T from Result<SessionResponse<T>>
fn extract_session_inner_type(ret: &ReturnType) -> Result<Type> {
    match ret {
        ReturnType::Default => Err(syn::Error::new(
            Span::call_site(),
            "Session module function must return Result<T>",
        )),
        ReturnType::Type(_, ty) => {
            if let Type::Path(type_path) = &**ty {
                if let Some(segment) = type_path.path.segments.last() {
                    if segment.ident == "Result" {
                        if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                            if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                                // inner_ty should be SessionResponse<T>
                                if let Type::Path(inner_path) = inner_ty {
                                    if let Some(seg) = inner_path.path.segments.last() {
                                        if seg.ident == "SessionResponse" {
                                            if let syn::PathArguments::AngleBracketed(inner_args) = &seg.arguments {
                                                if let Some(syn::GenericArgument::Type(output_ty)) = inner_args.args.first() {
                                                    return Ok(output_ty.clone());
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Err(syn::Error::new(
                Span::call_site(),
                "Session module must return Result<SessionResponse<T>>",
            ))
        }
    }
}


