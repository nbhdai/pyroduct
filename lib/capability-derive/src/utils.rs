use quote::quote;
use syn::{FnArg, Ident, Pat, ReturnType, Type, parse2, token::RArrow};

pub fn type_to_return(arg: &Type) -> ReturnType {
    ReturnType::Type(RArrow::default(), Box::new(arg.clone()))
} 

pub fn return_to_type(arg: &ReturnType) -> Type {
    match arg {
        ReturnType::Default => parse2(quote!(())).expect("Works"),
        ReturnType::Type(_, arg_type) => arg_type.as_ref().clone(),
    }
}

/// Extract parameter name from FnArg
pub fn get_param_name(arg: &FnArg) -> Option<Ident> {
    match arg {
        FnArg::Receiver(_) => None,
        FnArg::Typed(pat_type) => {
            if let Pat::Ident(pat_ident) = &*pat_type.pat {
                Some(pat_ident.ident.clone())
            } else {
                None
            }
        }
    }
}

/// Extract parameter type from FnArg
pub fn get_param_type(arg: &FnArg) -> Option<&Type> {
    match arg {
        FnArg::Receiver(_) => None,
        FnArg::Typed(pat_type) => Some(&*pat_type.ty),
    }
}


pub fn compare_types(a: &Type, b: &Type) -> bool {
    match (a, b) {
        (Type::Path(p1), Type::Path(p2)) => {
            // Compare paths segment by segment, ignoring spans
            p1.path.segments.len() == p2.path.segments.len() &&
            p1.path.segments.iter().zip(p2.path.segments.iter()).all(|(s1, s2)| {
                s1.ident == s2.ident && s1.arguments == s2.arguments
            })
        }
        // Fallback for non-path types
        _ => quote!(#a).to_string() == quote!(#b).to_string(),
    }
}

pub fn is_self_ref_or_type(input: &FnArg) -> bool {
    match input {
        FnArg::Receiver(_) => true,
        FnArg::Typed(pat_type) => {
            if let Type::Path(type_path) = &*pat_type.ty {
                if type_path.path.is_ident("Self") {
                    return true;
                }
            } else if let Type::Reference(type_ref) = &*pat_type.ty {
                if let Type::Path(type_path) = &*type_ref.elem {
                    if type_path.path.is_ident("Self") {
                        return true;
                    }
                }
            }
            false
        }
    }
}