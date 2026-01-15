use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, parse_macro_input};

use crate::deep_ref::map_type_to_ref;

pub fn derive(input: TokenStream) -> TokenStream {
    let import_location = super::import_path();
    let input = parse_macro_input!(input as DeriveInput);

    let struct_name = input.ident;
    let ref_struct_name = format_ident!("{}Ref", struct_name);

    let fields = match input.data {
        Data::Struct(ref data) => &data.fields,
        _ => panic!("FromRow can only be derived for structs"),
    };

    let mut lifetime_used = false;
    for f in fields {
        let (_, is_prim) = map_type_to_ref(&f.ty);
        if !is_prim {
            lifetime_used = true;
            break;
        }
    }

    let field_parsers = fields.iter().map(|f| {
        let name = f.ident.as_ref().unwrap();
        let name_str = name.to_string();
        let ty = &f.ty;
        let (mapped_type, _) = map_type_to_ref(ty);

        // We assume the struct definition exists (provided by DeepRef)
        // We use FromValue to convert the ArrowValue to the mapped type
        let error_msg = format!("Missing Field: {}", name_str);
        quote! {
            #name: <#mapped_type as #import_location::FromValue<'a>>::from_value(
                row.get(#name_str).ok_or(#error_msg.to_string())?
            )?
        }
    });

    let phantom_init = if !lifetime_used {
        quote! { _phantom: std::marker::PhantomData }
    } else {
        quote! {}
    };

    let expanded = quote! {
        impl<'a> #import_location::FromRow<'a> for #ref_struct_name<'a> {
            fn from_row(row: &#import_location::ArrowRow<'a>) -> Result<Self, String> {
                Ok(Self {
                    #(#field_parsers,)*
                    #phantom_init
                })
            }
        }
    };

    TokenStream::from(expanded)
}
