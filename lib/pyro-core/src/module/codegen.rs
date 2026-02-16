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

    for (name, ty) in &params {
        validate_ref_types(ty)
            .map_err(|e| syn::Error::new(name.span(), format!("in parameter `{}`: {}", name, e)))?;
    }

    // Extract return type (must be Result<T, String>)
    let return_type = extract_result_ok_type(&input_fn.sig.output)?;

    // Generate __Input struct fields (convert references to owned)
    let input_fields: Vec<_> = params
        .iter()
        .map(|(name, ty)| {
            let owned_ty = ref_to_owned(ty);
            quote! { #name: #owned_ty }
        })
        .collect();

    // Generate __Output struct and mapping based on output spec
    let (output_struct, output_mapping, output_name) =
        generate_output(&attrs.output, &return_type)?;

    // Generate the call arguments (extract from input struct)
    let call_args: Vec<_> = params
        .iter()
        .map(|(name, ty)| {
            if is_ref_type(ty) {
                quote! { &input.#name }
            } else {
                quote! { input.#name }
            }
        })
        .collect();

    // Generate the original function parameters
    let original_fn_params: Vec<_> = params
        .iter()
        .map(|(name, ty)| quote! { #name: #ty })
        .collect();

    let expanded = quote! {
        #[unsafe(no_mangle)]
        pub extern "C" fn exter_call(input_ptr: *mut u8) -> *const u8 {
            #[::pyroduct::magma]
            struct __Input {
                #(#input_fields,)*
            }

            #output_struct

            let call = |input: &__InputRef| {
                #fn_name(#(#call_args),*).map(|result| {
                    #output_mapping
                })
            };


            ::pyroduct::wasm::wasm::wasm_row_main::<__Input, #output_name, _>(input_ptr, call)
        }

        #(#fn_attrs)*
        #fn_vis fn #fn_name(#(#original_fn_params),*) -> Result<#return_type, String>
        #fn_block
    };

    Ok(expanded)
}

/// Extract the Ok type from Result<T, E>
fn extract_result_ok_type(ret: &ReturnType) -> Result<Type> {
    match ret {
        ReturnType::Default => Err(syn::Error::new(
            Span::call_site(),
            "Module function must return Result<T, String>",
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
                "Module function must return Result<T, String>",
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
                #[::pyroduct::magma]
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
                #[derive(::pyroduct::ToRow)]
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

/// Check if a type is a reference type
fn is_ref_type(ty: &Type) -> bool {
    matches!(ty, Type::Reference(_))
}

/// Convert reference types to owned types for struct fields
/// This handles the conversion from borrowed types in function signatures
/// to owned types in the generated __Input struct.
fn ref_to_owned(ty: &Type) -> TokenStream {
    if let Type::Reference(type_ref) = ty {
        let inner = &*type_ref.elem;

        // &str -> String
        if let Type::Path(type_path) = inner {
            if let Some(segment) = type_path.path.segments.last() {
                if segment.ident == "str" {
                    return quote! { String };
                }
            }
        }

        // &[T] -> Vec<T> (with T converted to owned if it's a Ref type)
        if let Type::Slice(slice) = inner {
            let elem = &slice.elem;
            let owned_elem = type_to_owned(elem);
            return quote! { Vec<#owned_elem> };
        }

        // Default: convert inner type to owned
        type_to_owned(inner)
    } else {
        // Non-reference type: convert to owned
        type_to_owned(ty)
    }
}

/// Convert a type to its owned equivalent
/// - SomeTypeRef<'_> -> SomeType (strips Ref suffix and lifetime)
/// - Primitive types stay as-is
/// - Other types stay as-is
fn type_to_owned(ty: &Type) -> TokenStream {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            let ident_str = segment.ident.to_string();

            // Check if the type ends with "Ref" and has lifetime parameters
            // e.g., CallMessageRef<'_> -> CallMessage
            if ident_str.ends_with("Ref") {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    // Check if it has lifetime arguments (like <'_> or <'a>)
                    let has_lifetime = args
                        .args
                        .iter()
                        .any(|arg| matches!(arg, syn::GenericArgument::Lifetime(_)));

                    if has_lifetime {
                        // Strip the "Ref" suffix to get the owned type name
                        let owned_name = &ident_str[..ident_str.len() - 3];
                        let owned_ident = Ident::new(owned_name, segment.ident.span());
                        return quote! { #owned_ident };
                    }
                }
            }
        }
    }

    // Default: return the type as-is
    quote! { #ty }
}

fn validate_ref_types(ty: &Type) -> Result<()> {
    validate_ref_types_inner(ty, ty)
}

fn validate_ref_types_inner(ty: &Type, original_ty: &Type) -> Result<()> {
    match ty {
        Type::Path(type_path) => {
            if let Some(segment) = type_path.path.segments.last() {
                let ident_str = segment.ident.to_string();

                // Check if type name ends with "Ref" (like CallMessageRef)
                if ident_str.ends_with("Ref") && ident_str.len() > 3 {
                    // Check if it has angle bracket arguments with a lifetime
                    let has_lifetime = match &segment.arguments {
                        syn::PathArguments::AngleBracketed(args) => args
                            .args
                            .iter()
                            .any(|arg| matches!(arg, syn::GenericArgument::Lifetime(_))),
                        _ => false,
                    };

                    if !has_lifetime {
                        let base_name = &ident_str[..ident_str.len() - 3];
                        return Err(syn::Error::new_spanned(
                            original_ty,
                            format!(
                                "`{ident_str}` requires a lifetime parameter. \
                                Use `{ident_str}<'_>` for an inferred lifetime, or define your \
                                input as `{base_name}` (owned) and let the macro handle the conversion."
                            ),
                        ));
                    }
                }

                // Recursively check generic arguments (e.g., Vec<SomeRef>)
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    for arg in &args.args {
                        if let syn::GenericArgument::Type(inner_ty) = arg {
                            validate_ref_types_inner(inner_ty, original_ty)?;
                        }
                    }
                }
            }
        }
        Type::Reference(type_ref) => {
            // Check the inner type of references (e.g., &[SomeRef])
            validate_ref_types_inner(&type_ref.elem, original_ty)?;
        }
        Type::Slice(slice) => {
            // Check slice element type
            validate_ref_types_inner(&slice.elem, original_ty)?;
        }
        Type::Tuple(tuple) => {
            // Check all tuple elements
            for elem in &tuple.elems {
                validate_ref_types_inner(elem, original_ty)?;
            }
        }
        Type::Array(array) => {
            validate_ref_types_inner(&array.elem, original_ty)?;
        }
        _ => {}
    }
    Ok(())
}
