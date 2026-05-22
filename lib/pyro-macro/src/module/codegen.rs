use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{FnArg, ItemFn, Pat, Result, ReturnType, Type, parse_quote};

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
                    let attrs = pat_type.attrs.clone();
                    let pat = pat_type.pat.clone();
                    return Some((name, ty, attrs, pat));
                }
            }
            None
        })
        .collect();

    // Validate module requirements and extract types
    let return_type = extract_result_ok_type(&input_fn.sig.output)?;

    // Generate __Output struct and mapping based on output spec
    let (output_struct, output_mapping, output_name) =
        generate_output(&attrs.output, &return_type)?;

    // Generate the call arguments (extract from input struct)
    let call_args: Vec<_> = params
        .iter()
        .map(|(name, ty, _, _)| {
            let name_str = name.to_string();
            quote! { input.get_value::<#ty>(#name_str).ok_or_else(|| ::pyroduct::CapturedError::new(format!("Missing {}", #name_str)))? }
        })
        .collect();

    // Generate the original function parameters
    let original_fn_params: Vec<_> = params
        .iter()
        .map(|(_, ty, attrs, pat)| quote! { #(#attrs)* #pat: #ty })
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

fn wrap_in_vec(ty: &Type) -> Type {
    parse_quote!(Vec<#ty>)
}

pub fn expand_session(attrs: ModuleAttrs, input_fn: ItemFn) -> Result<TokenStream> {
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
                    let attrs = pat_type.attrs.clone();
                    let pat = pat_type.pat.clone();
                    return Some((name, ty, attrs, pat));
                }
            }
            None
        })
        .collect();

    // Generate __Output struct and mapping based on output spec
    let output_type = extract_session_inner_type(&input_fn.sig.output)?;
    let output_vec = wrap_in_vec(&output_type);
    let output_spec = attrs.output;
    let (output_struct, output_mapping, output_name) = generate_output(&output_spec, &output_type)?;

    // Generate the original function parameters
    let original_fn_params: Vec<_> = params
        .iter()
        .map(|(_, ty, attrs, pat)| quote! { #(#attrs)* #pat: #ty })
        .collect();

    // Validate session module requirements and extract types
    let expanded = match params.len() {
        2 => {
            let input_vec = wrap_in_vec(&params[1].1);
            if !(params[0].0 == "prior" || params[0].0 == "_prior") {
                return Err(syn::Error::new(
                    params[0].0.span(),
                    "If 2 inputs, then the first parameter of session module must be named `prior`",
                ));
            }

            if params[1].1 != output_type {
                return Err(syn::Error::new(
                    params[1].0.span(),
                    "The type of the output must be the same as input and prior",
                ));
            }

            if params[0].1 != input_vec || params[0].1 != output_vec {
                return Err(syn::Error::new(
                    params[0].0.span(),
                    format!("The type of the prior must be type: {:?}", input_vec),
                ));
            }

            quote! {
                #[unsafe(no_mangle)]
                pub extern "C" fn call_session_extern(session_id: u32) -> *const u8 {
                    #output_struct

                    let call = |prior: &[::pyroduct::PyroRow<'_>], input: ::pyroduct::PyroRow<'_>| {
                        let prior = prior.iter().map(|p| p.clone().try_into()).collect::<Result<Vec<#output_type>, _>>().map_err(|e| {
                            ::pyroduct::CapturedError::new("Unable to extract prior data")
                                .with_source(e)
                        })?;
                        let input = input.try_into().map_err(|e| {
                            ::pyroduct::CapturedError::new("Unable to extract input data")
                                .with_source(e)
                        })?;
                        #fn_name(prior, input).map(|result| {
                            match result {
                                ::pyroduct::session::SessionResponse::Continue(result) => {
                                    ::pyroduct::session::SessionResponse::Continue(#output_mapping)
                                }
                                ::pyroduct::session::SessionResponse::End(result) => {
                                    ::pyroduct::session::SessionResponse::End(#output_mapping)
                                }
                                ::pyroduct::session::SessionResponse::Terminate => {
                                    ::pyroduct::session::SessionResponse::Terminate
                                }
                            }
                        })
                    };

                    ::pyroduct::wasm::wasm_row_main_session::<#output_name, _>(session_id, call)
                }

                #(#fn_attrs)*
                #fn_vis fn #fn_name(#(#original_fn_params),*) -> ::pyroduct::wasm::ModuleResult<::pyroduct::session::SessionResponse<#output_type>>
                #fn_block
            }
        }
        3 => {
            if !(params[0].0 == "prior_input" || params[0].0 == "_prior_input") {
                return Err(syn::Error::new(
                    params[0].0.span(),
                    "If 3 inputs, then the first parameter of session module must be named `prior_input`",
                ));
            }
            if !(params[1].0 == "prior_output" || params[1].0 == "_prior_output") {
                return Err(syn::Error::new(
                    params[1].0.span(),
                    "If 3 inputs, then the second parameter of session module must be named `prior_output`",
                ));
            }
            let input_type = &params[2].1;
            let input_vec = wrap_in_vec(&params[2].1);
            if params[0].1 != input_vec {
                return Err(syn::Error::new(
                    params[0].0.span(),
                    format!(
                        "First parameter of session module must have the type: {:?}",
                        input_vec
                    ),
                ));
            }
            if params[1].1 != output_vec {
                return Err(syn::Error::new(
                    params[1].0.span(),
                    format!(
                        "Second parameter of session module must have the type: {:?}",
                        output_type
                    ),
                ));
            }

            quote! {
                #[unsafe(no_mangle)]
                pub extern "C" fn call_session_extern(session_id: u32) -> *const u8 {
                    #output_struct

                    let call = |prior_inputs: &[::pyroduct::PyroRow<'_>], prior_outputs: &[::pyroduct::PyroRow<'_>], input: ::pyroduct::PyroRow<'_>| {
                        let prior_inputs = prior_inputs.iter().map(|p| p.clone().try_into()).collect::<Result<Vec<#input_type>, _>>().map_err(|e| {
                            ::pyroduct::CapturedError::new("Unable to extract prior input data")
                                .with_source(e)
                        })?;
                        let prior_outputs = prior_outputs.iter().map(|p| p.clone().try_into()).collect::<Result<Vec<#output_type>, _>>().map_err(|e| {
                            ::pyroduct::CapturedError::new("Unable to extract prior output data")
                                .with_source(e)
                        })?;
                        let input = input.try_into().map_err(|e| {
                            ::pyroduct::CapturedError::new("Unable to extract input data")
                                .with_source(e)
                        })?;
                        #fn_name(prior_inputs, prior_outputs, input).map(|result| {
                            match result {
                                ::pyroduct::session::SessionResponse::Continue(result) => {
                                    ::pyroduct::session::SessionResponse::Continue(#output_mapping)
                                }
                                ::pyroduct::session::SessionResponse::End(result) => {
                                    ::pyroduct::session::SessionResponse::End(#output_mapping)
                                }
                                ::pyroduct::session::SessionResponse::Terminate => {
                                    ::pyroduct::session::SessionResponse::Terminate
                                }
                            }
                        })
                    };

                    ::pyroduct::wasm::wasm_row_main_session_diff::<#output_name, _>(session_id, call)
                }

                #(#fn_attrs)*
                #fn_vis fn #fn_name(#(#original_fn_params),*) -> ::pyroduct::wasm::ModuleResult<::pyroduct::session::SessionResponse<#output_type>>
                #fn_block
            }
        }
        _ => {
            return Err(syn::Error::new(
                Span::call_site(),
                "Session module functions must have either 3 parameters (prior_input, prior_output, and input), or 2 parameters (prior, and input) with the same type for input and output",
            ));
        }
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

            let mapping = quote! {
                __Output {
                    #field_name: result,
                }
            };

            let output_name = parse_quote!(__Output);

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

        // Pattern 3: Existing struct that implements ToRow
        OutputSpec::Struct => Ok((quote! {}, quote! { result }, return_type.clone())),
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
                                            if let syn::PathArguments::AngleBracketed(inner_args) =
                                                &seg.arguments
                                            {
                                                if let Some(syn::GenericArgument::Type(output_ty)) =
                                                    inner_args.args.first()
                                                {
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
