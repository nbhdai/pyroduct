use quote::quote;
use syn::{FnArg, Ident, Pat, ReturnType, Type, parse_quote};

/// Get the return type, defaulting to () if none specified
pub fn get_return_type(ret: &ReturnType) -> Type {
    match &ret {
        ReturnType::Default => {
            tracing::trace!("No return type specified, defaulting to unit ()");
            parse_quote!(())
        }
        ReturnType::Type(_rarrow, return_type) => {
            let rt = return_type.as_ref().clone();
            tracing::trace!("Detected return type: {}", quote!(#rt));
            rt
        }
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
