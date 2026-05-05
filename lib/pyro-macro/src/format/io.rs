use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{GenericParam, ItemStruct, Path};

pub fn bridgeable(input: &ItemStruct, import_location: &Path) -> syn::Result<TokenStream> {
    let struct_name = &input.ident;
    let ref_struct_name = format_ident!("{}Ref", struct_name);
    let encoder_name = format_ident!("{}Encoder", struct_name);
    let decoder_name = format_ident!("{}Decoder", struct_name);

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    // Prepare arguments for the Ref struct using 'a
    let mut ref_struct_args = Vec::new();
    ref_struct_args.push(quote! { 'decoder_life });

    for param in &input.generics.params {
        match param {
            GenericParam::Type(t) => {
                let ident = &t.ident;
                ref_struct_args
                    .push(quote! { <#ident as #import_location::format::DeepRef>::Ref<'_> });
            }
            GenericParam::Const(c) => {
                let ident = &c.ident;
                ref_struct_args.push(quote! { #ident });
            }
            GenericParam::Lifetime(l) => {
                let ident = &l.lifetime;
                ref_struct_args.push(quote! { #ident });
            }
        }
    }

    let expanded = quote! {
        #[derive(Default)]
        pub struct #encoder_name #impl_generics #where_clause {
             _phantom: std::marker::PhantomData<#struct_name #ty_generics>,
        }

        impl #impl_generics #import_location::format::Encoder<#struct_name #ty_generics> for #encoder_name #ty_generics #where_clause {
            fn encode(&mut self, val: &#struct_name #ty_generics) -> Result<#import_location::format::PyroVec, #import_location::PyroError> {
                let row = #import_location::format::ToRow::to_row(val);
                let mut wire = #import_location::format::PyroValue::Group(row).to_wire()?;
                <#import_location::format::PyroVec as #import_location::format::header::PyroHeaderMut>::set_status(&mut wire, #import_location::format::header::DataStatus::Valid);
                Ok(wire)
            }
        }

        #[derive(Default)]
        pub struct #decoder_name #impl_generics #where_clause {
             _phantom: std::marker::PhantomData<#struct_name #ty_generics>,
        }

        impl<'decoder_life> #impl_generics #import_location::format::Decoder<'decoder_life, #ref_struct_name < #(#ref_struct_args),* >> for #decoder_name #ty_generics #where_clause {
            fn decode(&mut self, view: &'decoder_life #import_location::format::PyroRef<'decoder_life>) -> Result<#ref_struct_name < #(#ref_struct_args),* >, #import_location::PyroError> {
                let val = #import_location::format::PyroValue::parse_wire(view)?;
                if let #import_location::format::PyroValue::Group(row) = val {
                    #ref_struct_name::try_from(row).map_err(|_| #import_location::PyroError::deserialization(
                        Box::new(#import_location::CapturedError::new(format!("Failed to parse {}", stringify!(#struct_name)))
                            .with_location(::std::panic::Location::caller()))
                    ))
                } else {
                     Err(#import_location::PyroError::deserialization(
                        Box::new(#import_location::CapturedError::new(format!("Expected Group, found {:?}", val))
                            .with_location(::std::panic::Location::caller()))
                    ))
                }
            }
        }

        impl #impl_generics #import_location::format::Bridgeable for #struct_name #ty_generics #where_clause {
            type Encoder = #encoder_name #ty_generics;
            type Decoder = #decoder_name #ty_generics;
            type Ref<'decoder_life> = #ref_struct_name < #(#ref_struct_args),* >;
        }
    };

    Ok(expanded)
}
