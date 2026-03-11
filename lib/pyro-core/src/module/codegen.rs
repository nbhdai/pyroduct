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
                    return Some((name, ty));
                }
            }
            None
        })
        .collect();

    // Extract return type (must be Result<T, String>)
    let return_type = extract_result_ok_type(&input_fn.sig.output)?;

    // Generate __Output struct and mapping based on output spec
    let (output_struct, output_mapping, output_name) =
        generate_output(&attrs.output, &return_type)?;

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

            Ok((struct_def, mapping, parse_quote!(__Output)))
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

            let field_mappings: Vec<_> = field_names
                .iter()
                .enumerate()
                .map(|(i, name)| {
                    let idx = syn::Index::from(i);
                    quote! { #name: result.#idx }
                })
                .collect();

            let struct_def = quote! {
                #[derive(::pyroduct::format::ToRow, ::pyroduct::format::Document)]
                struct __Output {
                    #(#field_defs,)*
                }
            };

            let mapping = quote! {
                __Output {
                    #(#field_mappings,)*
                }
            };

            Ok((struct_def, mapping, parse_quote!(__Output)))
        }

        // Pattern 3: Existing struct that implements ToRow
        OutputSpec::Struct => {
            // No __Output struct needed, use the return type directly
            let struct_def = quote! {};

            // Just pass through - the struct already implements ToRow
            let mapping = quote! { result };

            Ok((struct_def, mapping, return_type.clone()))
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
