use quote::quote;
use syn::{
    Error, FnArg, Ident, PathArguments, Result, ReturnType, Type, parse2,
    token::RArrow,
};

pub fn type_to_return(arg: &Type) -> ReturnType {
    ReturnType::Type(RArrow::default(), Box::new(arg.clone()))
}

pub fn return_to_type(arg: &ReturnType) -> Type {
    match arg {
        ReturnType::Default => parse2(quote!(())).expect("Works"),
        ReturnType::Type(_, arg_type) => arg_type.as_ref().clone(),
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

pub fn extract_ident_from_type(ty: &Type) -> Result<Ident> {
    match ty {
        // We only accept Type::Path (e.g., "MyStruct")
        Type::Path(type_path) => {
            // 1. Reject Qualified Self
            // Blocks: <MyStruct as SomeTrait>::AssocType
            if type_path.qself.is_some() {
                return Err(Error::new_spanned(
                    type_path,
                    "Type must be a simple identifier, not a qualified path.",
                ));
            }

            // 2. Enforce "1 word" rule (No module path)
            // Blocks: crate::MyStruct, std::vec::Vec
            if type_path.path.segments.len() != 1 {
                return Err(Error::new_spanned(
                    &type_path.path,
                    "Type must be a simple identifier, not a module path (e.g., use 'MyType', not 'crate::MyType').",
                ));
            }

            // Safe to unwrap because len == 1
            let segment = type_path.path.segments.first().unwrap();

            // 3. Enforce no generics
            // Blocks: MyStruct<T>, MyStruct<'a>
            if !matches!(segment.arguments, PathArguments::None) {
                return Err(Error::new_spanned(
                    &type_path.path,
                    "Type must not have generic parameters (e.g., use 'MyType', not 'MyType<T>').",
                ));
            }

            Ok(segment.ident.clone())
        }

        // 4. Reject all other types (References, Pointers, Arrays, Tuples, etc.)
        _ => Err(Error::new_spanned(
            ty,
            "Type must be a simple struct identifier. References, tuples, slices, or pointers are not allowed.",
        )),
    }
}

pub fn extract_ident_ignoring_ref(ty: &Type) -> Option<&Ident> {
    // 1. Peel off the reference (if it exists) to get the inner type
    // Handles &MyClient -> MyClient
    let inner_ty = if let Type::Reference(type_ref) = ty {
        &type_ref.elem
    } else {
        ty
    };

    // 2. Check if the inner type is a Path (e.g. MyClient, crate::MyClient)
    if let Type::Path(type_path) = inner_ty {
        // 3. Ensure no Qualified Self (<Type as Trait>::Assoc)
        if type_path.qself.is_some() {
            return None;
        }

        // 4. Enforce "1 word" rule
        // Accepts: MyClient
        // Rejects: crate::MyClient, foo::MyClient
        if type_path.path.segments.len() != 1 {
            return None;
        }

        let segment = type_path.path.segments.first().unwrap();

        // 5. Enforce no generics
        // Accepts: MyClient
        // Rejects: MyClient<T>
        if matches!(segment.arguments, PathArguments::None) {
            return Some(&segment.ident);
        }
    }

    // Returns None for everything else (Tuples, Arrays, complex paths, etc.)
    None
}
