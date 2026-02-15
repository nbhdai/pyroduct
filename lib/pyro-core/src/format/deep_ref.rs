use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::{
    GenericParam, Ident, ItemStruct, Lifetime, LifetimeParam, Path, PathSegment, TraitBound, Type,
    WherePredicate, parse_quote,
};

pub fn deep_ref(
    input: &ItemStruct,
    import_location: &Path,
    derives_to_pass: &Vec<Ident>,
) -> syn::Result<TokenStream> {
    // let input: DeriveInput = syn::parse2(input)?;

    let struct_name = input.ident.clone();
    let ref_struct_name = format_ident!("{}Ref", struct_name);

    // 1. Prepare Generics for the Ref Struct Definition
    // We want: struct PersonRef<'deep_ref_lifetime, TRef>
    let mut ref_struct_generics = syn::Generics::default();

    // Always add the lifetime first
    let lifetime = Lifetime::new("'deep_ref_lifetime", Span::call_site());
    ref_struct_generics
        .params
        .push(GenericParam::Lifetime(LifetimeParam::new(lifetime.clone())));

    // Map original generic params (T) to new ref params (TRef)
    let mut param_map = std::collections::HashMap::new();

    for param in &input.generics.params {
        match param {
            GenericParam::Type(type_param) => {
                let old_ident = &type_param.ident;
                let new_ident = format_ident!("{}Ref", old_ident);

                // Store mapping T -> TRef
                param_map.insert(old_ident.clone(), new_ident.clone());

                // Add TRef to the Ref struct generics
                ref_struct_generics
                    .params
                    .push(GenericParam::Type(syn::TypeParam::from(new_ident)));
            }
            GenericParam::Const(const_param) => {
                ref_struct_generics
                    .params
                    .push(GenericParam::Const(const_param.clone()));
            }
            GenericParam::Lifetime(lp) => {
                ref_struct_generics
                    .params
                    .push(GenericParam::Lifetime(lp.clone()));
            }
        }
    }

    let mut ref_fields = Vec::with_capacity(input.fields.len());

    for f in &input.fields {
        let name = &f.ident;
        let vis = &f.vis;
        let ty = &f.ty;

        let mut replaced = false;
        if let syn::Type::Path(type_path) = ty {
            if type_path.qself.is_none() && type_path.path.segments.len() == 1 {
                let seg = &type_path.path.segments[0];
                if let Some(new_ident) = param_map.get(&seg.ident) {
                    ref_fields.push(quote! { #vis #name: #new_ident });
                    replaced = true;
                }
            }
        }

        if !replaced {
            ref_fields
                .push(quote! { #vis #name: <#ty as #import_location::DeepRef>::Ref<#lifetime> });
        }
    }

    let (phantom_field, phantom_init) = if ref_fields.is_empty() {
        (
            quote! { _phantom: std::marker::PhantomData<&'deep_ref_lifetime ()> },
            quote! { _phantom: std::marker::PhantomData },
        )
    } else {
        (quote! {}, quote! {})
    };

    let derives = if derives_to_pass.is_empty() {
        quote! {}
    } else {
        quote! { #[derive(#(#derives_to_pass,)*)] }
    };

    let struct_def = quote! {
        #derives
        pub struct #ref_struct_name #ref_struct_generics {
            #(#ref_fields,)*
            #phantom_field
        }
    };

    // 3. Prepare Generics for the Impl Block
    let mut impl_generics = input.generics.clone();
    let mut deep_ref_bound_path = import_location.clone();
    deep_ref_bound_path
        .segments
        .push(PathSegment::from(format_ident!("DeepRef")));

    // Add DeepRef bound to all original type parameters
    for param in impl_generics.params.iter_mut() {
        if let GenericParam::Type(t) = param {
            t.bounds.push(syn::TypeParamBound::Trait(TraitBound {
                paren_token: None,
                modifier: syn::TraitBoundModifier::None,
                lifetimes: None,
                path: deep_ref_bound_path.clone(),
            }));
        }
    }

    let (impl_g, ty_g, where_clause) = impl_generics.split_for_impl();

    let mut ref_struct_args = Vec::new();
    ref_struct_args.push(quote! { #lifetime });

    let mut associated_type_bounds = Vec::new();

    for param in &input.generics.params {
        match param {
            GenericParam::Type(t) => {
                let ident = &t.ident;
                ref_struct_args
                    .push(quote! { <#ident as #import_location::DeepRef>::Ref<#lifetime> });

                // Add the bound: T: 'deep_ref_lifetime
                associated_type_bounds.push(quote! { #ident: #lifetime });
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

    // 5. Conversions
    let mut field_conversions = Vec::with_capacity(input.fields.len());
    for f in &input.fields {
        let field_name = f
            .ident
            .as_ref()
            .ok_or_else(|| syn::Error::new_spanned(f, "DeepRef requires named fields"))?;
        let ty = &f.ty;
        field_conversions.push(quote! { #field_name: <#ty as #import_location::DeepRef>::as_deep_ref(&self.#field_name) });
    }

    // Added the where clause specifically to the associated type definition
    let impl_owned = quote! {
        impl #impl_g #import_location::DeepRef for #struct_name #ty_g #where_clause {
            type Ref<'deep_ref_lifetime> = #ref_struct_name < #(#ref_struct_args),* >
            where #(#associated_type_bounds),*;

            fn as_deep_ref(&self) -> Self::Ref<'_> {
                #ref_struct_name {
                    #(#field_conversions,)*
                    #phantom_init
                }
            }
        }
    };

    let expanded = quote! {
        #struct_def
        #impl_owned
    };

    Ok(TokenStream::from(expanded))
}

pub fn deep_ref_rkyv(input: &ItemStruct, import_location: &Path) -> syn::Result<TokenStream> {
    let struct_name = &input.ident;
    let ref_struct_name = format_ident!("{}Ref", struct_name);

    // 1. Setup Paths
    // The prompt implies import_location is the root (e.g., ::pyroduct),
    // so we derive rkyv location from it.
    let deep_ref_path = &import_location;
    let rkyv_path = quote! { #import_location::rkyv_8::rkyv };

    // 2. Prepare Generics for the Impl Block
    // We need to modify the generics of the original struct to add bounds:
    // T: DeepRef + Archive
    let mut impl_generics = input.generics.clone();
    let mut where_clause = impl_generics.make_where_clause().clone();
    let mut ty_g: Vec<Type> = Vec::new();

    // The lifetime for the HRTB (Higher-Rank Trait Bound)
    let lifetime = Lifetime::new("'deep_ref_lifetime", Span::call_site());

    for param in impl_generics.params.iter_mut() {
        if let GenericParam::Type(t) = param {
            let ident = &t.ident;

            // Add T: DeepRef + Archive bounds
            t.bounds.push(parse_quote!(#deep_ref_path::DeepRef));
            t.bounds.push(parse_quote!(#rkyv_path::Archive));

            ty_g.push(parse_quote!(#rkyv_path::Archived<#ident>));

            // Add the complex GAT bound:
            // for<'deep_ref_lifetime> <T as Archive>::Archived: DeepRef<Ref = <T as DeepRef>::Ref<'deep_ref_lifetime>>
            // Note: We use parse_quote to construct this complex predicate
            let predicate: WherePredicate = parse_quote! {
                for<#lifetime> <#rkyv_path::Archived<#ident>: #deep_ref_path::DeepRef<Ref<'deep_ref_lifetime> = <#ident as #deep_ref_path::DeepRef>::Ref<#lifetime>>
            };
            where_clause.predicates.push(predicate);
        }
    }

    let (impl_g, _, _) = impl_generics.split_for_impl();

    // 3. Prepare Arguments for the Ref struct (UserRef<...>)
    // The Ref struct needs: 'deep_ref_lifetime, followed by mapped types
    let mut ref_struct_args = Vec::new();
    ref_struct_args.push(quote! { #lifetime });

    // We also need bounds for the associated type definition
    let mut associated_type_bounds = Vec::new();

    for param in &input.generics.params {
        match param {
            GenericParam::Type(t) => {
                let ident = &t.ident;
                // Map T -> <T as DeepRef>::Ref<'deep_ref_lifetime>
                ref_struct_args
                    .push(quote! { <#ident as #deep_ref_path::DeepRef>::Ref<#lifetime> });

                // Add the bound T: 'deep_ref_lifetime to the associated type where clause
                associated_type_bounds.push(
                    quote! { <#ident as ::pyroduct::rkyv_8::rkyv::Archive>::Archived: #lifetime },
                );
                // associated_type_bounds.push(quote! { #ident : #lifetime });
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

    let mut field_conversions = Vec::with_capacity(input.fields.len());
    for f in &input.fields {
        let name = f
            .ident
            .as_ref()
            .ok_or_else(|| syn::Error::new_spanned(f, "DeepRef requires named fields"))?;
        let ty = &f.ty;

        // conversion logic:
        // <<Type as Archive>::Archived as DeepRef>::as_deep_ref(&self.field)
        field_conversions.push(quote! {
            #name: <<#ty as #rkyv_path::Archive>::Archived as #deep_ref_path::DeepRef>::as_deep_ref(&self.#name)
        });
    }

    // Handle PhantomData initialization if the struct has no fields
    // (Matching the behavior of the provided deep_ref function)
    let phantom_init = if field_conversions.is_empty() {
        quote! { _phantom: std::marker::PhantomData }
    } else {
        quote! {}
    };

    // 5. Final Impl Construction
    // Target: <Struct<T> as Archive>::Archived
    let target_type = quote! { #rkyv_path::Archived<#struct_name <#(#ty_g),*> > };

    let expanded = quote! {
        impl #impl_g #deep_ref_path::DeepRef for #target_type #where_clause {
            type Ref<'deep_ref_lifetime> = #ref_struct_name < #(#ref_struct_args),* >
            where #(#associated_type_bounds),*;

            fn as_deep_ref(&self) -> Self::Ref<'_> {
                #ref_struct_name {
                    #(#field_conversions,)*
                    #phantom_init
                }
            }
        }
    };

    Ok(TokenStream::from(expanded))
}
