use quote::quote;
use syn::{FnArg, Ident, Pat, ReturnType, Type, parse_quote};

/// Get the return type, defaulting to () if none specified
pub fn get_return_type(ret: &ReturnType) -> Type {
     match &ret {
        ReturnType::Default => {
            tracing::trace!("No return type specified, defaulting to unit ()");
            parse_quote!(())
        },
        ReturnType::Type(_rarrow, return_type) => {
            let rt = return_type.as_ref().clone();
            tracing::trace!("Detected return type: {}", quote!(#rt));
            rt
        },
    }
}

/// Check if a parameter has the #[client_state] attribute
pub fn has_client_state_attr(arg: &FnArg) -> bool {
    if let FnArg::Typed(pat_type) = arg {
        pat_type
            .attrs
            .iter()
            .any(|attr| attr.path().is_ident("client_state"))
    } else {
        false
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

/// Strip reference from type if present
pub fn strip_reference(ty: &Type) -> &Type {
    if let Type::Reference(type_ref) = ty {
        &*type_ref.elem
    } else {
        ty
    }
}
