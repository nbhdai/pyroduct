use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Ident, ItemStruct, Meta, Path, Token,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token::Comma,
};

/// Defines the documentation requirements for the configuration struct.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocRec {
    /// No documentation required.
    NoReq,
    /// The top-level struct must be documented.
    StructDoc,
    /// The struct and all its fields must be documented.
    AllDoc,
}

impl DocRec {
    pub fn need_struct(&self) -> bool {
        match self {
            DocRec::NoReq => false,
            DocRec::StructDoc => true,
            DocRec::AllDoc => true,
        }
    }
}

/// Whitelist of derives that are safe to forward to the rkyv `Archived` type.
const DERIVE_WHITELIST: &[&str] = &["Debug", "PartialEq", "Eq", "PartialOrd", "Ord"];

/// Parsed arguments for the `#[bridgeable(...)]` attribute macro.
///
/// Supports the following syntax:
///
/// ```text
/// #[bridgeable]                                    // defaults
/// #[bridgeable(Document)]                          // enable Documented impl
/// #[bridgeable(derive(Debug, PartialEq))]          // forward derives
/// #[bridgeable(derive(Debug, PartialEq), Document)] // both
/// ```
///
/// - `derive(...)` — derives to forward to the archived type (whitelisted to
///   Debug, PartialEq, Eq, PartialOrd, Ord). PartialEq and PartialOrd also
///   generate `#[rkyv(compare(...))]` attributes.
/// - `Document` — opt-in flag that generates a `Documented` impl producing a
///   JSON type spec for FFI/RPC consumers.
pub struct BridgeableArgs {
    /// Whitelisted derives to pass through to the rkyv `Archived` type.
    pub derives_to_pass: Vec<Ident>,
    /// Comparison traits (PartialEq / PartialOrd) found in the derives —
    /// these also become `#[rkyv(compare(...))]` entries.
    pub compares_to_add: Vec<Ident>,
    pub doc_rec: DocRec,
}

impl BridgeableArgs {
    /// Build from an already-parsed `Punctuated<Meta, Comma>` list.
    /// This is the shared logic used by both `Parse` and a direct call site.
    pub fn from_metas(metas: Punctuated<Meta, Comma>) -> syn::Result<Self> {
        let mut derives_to_pass = Vec::new();
        let mut compares_to_add = Vec::new();

        for meta in metas {
            match &meta {
                // derive(Debug, PartialEq, ...)
                Meta::List(list) if list.path.is_ident("derive") => {
                    let nested = list
                        .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
                        .unwrap_or_default();

                    for nested_meta in nested {
                        if let Meta::Path(path) = nested_meta {
                            if let Some(ident) = path.get_ident() {
                                let s = ident.to_string();

                                if DERIVE_WHITELIST.contains(&s.as_str()) {
                                    derives_to_pass.push(ident.clone());
                                }

                                if s == "PartialEq" || s == "PartialOrd" {
                                    compares_to_add.push(ident.clone());
                                }
                            }
                        }
                    }
                }
                other => {
                    return Err(syn::Error::new_spanned(
                        other,
                        "unexpected bridgeable argument; expected `derive(...)` or `Document`",
                    ));
                }
            }
        }

        Ok(BridgeableArgs {
            derives_to_pass,
            compares_to_add,
            doc_rec: DocRec::NoReq,
        })
    }
}

/// Allows `parse_macro_input!(args as BridgeableArgs)` in proc-macro crates.
impl Parse for BridgeableArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let metas = Punctuated::<Meta, Token![,]>::parse_terminated(input)?;
        Self::from_metas(metas)
    }
}

//pub mod bridgeable;
pub mod deep_ref;
pub mod documentation;
pub mod from_row;
pub mod io;
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

    // 2. Generate DeepRef
    let deep_ref = deep_ref::deep_ref(&item, import_location, &args.derives_to_pass)?;

    // 3. Generate FromRow
    let from_row = from_row::from_row(&item, import_location)?;
    let ref_from_row = from_row::ref_from_row(&item, import_location)?;

    // 4. Generate ToRow
    let to_row = to_row::to_row(&item, import_location)?;

    // 5. Generate Bridgeable
    let bridgeable = io::bridgeable(&item, import_location)?;

    Ok(quote! {
        #item
        #documentation
        #ref_documentation
        #deep_ref
        #from_row
        #ref_from_row
        #to_row
        #bridgeable
    })
}
