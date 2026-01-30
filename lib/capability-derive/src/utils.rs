use quote::quote;
use syn::{
    Error, Ident, PathArguments, Result, ReturnType, Type, parse2,
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
