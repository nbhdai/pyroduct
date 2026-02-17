use proc_macro2::TokenStream;
use quote::quote;
use syn::{ItemStruct, Path};

// Only import types we need; functions are accessed via their modules
use crate::format::bridgeable::BridgeableArgs;

pub mod bridgeable;
pub mod deep_ref;
pub mod documentation;
pub mod from_row;
pub mod library;
pub mod to_row;

pub fn magma(
    args: BridgeableArgs,
    item: &mut ItemStruct,
    import_location: &Path,
) -> syn::Result<TokenStream> {
    // 1. Generate Documentation (takes &Item, &Path)
    let documentation =
        documentation::generate_documented_impl(&item, &import_location, args.doc_rec)?;
    let ref_documentation =
        documentation::ref_documentation(&item, &import_location, args.doc_rec)?;

    // 2. Generate Bridgeable
    // Note: This consumes 'args' and 'item', so we use clones for the others.
    let bridge_tokens = bridgeable::bridgeable(&args, item, import_location)?;

    // 3. Prepare TokenStream for macros that parse their own input

    // 4. Generate DeepRef
    // Note: The current deep_ref implementation does not support 'derives_to_pass'
    let deep_ref = deep_ref::deep_ref(&item, import_location, &args.derives_to_pass)?;

    // 5. Generate DeepRef Rkyv
    let deep_ref_rkyv = deep_ref::deep_ref_rkyv(&item, import_location)?;

    // 6. Generate FromRow
    let from_row = from_row::from_row(&item, import_location)?;
    let ref_from_row = from_row::ref_from_row(&item, import_location)?;

    // 7. Generate ToRow
    let to_row = to_row::to_row(&item, import_location)?;

    Ok(quote! {
        #documentation
        #ref_documentation
        #bridge_tokens
        #deep_ref
        #deep_ref_rkyv
        #from_row
        #ref_from_row
        #to_row
    })
}
