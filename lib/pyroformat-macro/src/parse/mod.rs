pub(crate) mod type_map;

use pyro_schema::Word;
pub(crate) use type_map::{Leaf, Prim, ResolvedType};

use syn::{Data, DeriveInput, Error, Fields, Result};

/// The one struct shape every derive in this crate works from.
///
/// Opaque outside this crate: `pyroformat-derive` only parses one and hands
/// it to [`crate::layout::expand`], [`crate::expand_writer`] and
/// [`crate::reader::expand`].
#[derive(Debug, Clone)]
pub struct StructInput {
    pub(crate) name: syn::Ident,
    /// The path every generated item is reached through: `::pyroformat`, or
    /// whatever `#[pyro(import = "...")]` on the struct names, for a crate
    /// that re-exports `pyroformat` under its own name.
    pub(crate) krate: proc_macro2::TokenStream,
    pub(crate) vis: syn::Visibility,
    /// The struct's `///` documentation, or empty.
    pub(crate) doc: String,
    /// In declaration order, which is also inode order.
    pub(crate) fields: Vec<Field>,
    /// The word size we're using.
    pub(crate) word: Word,
}

impl StructInput {
    /// Reach `pyroformat` through `krate` (e.g. `::pyroduct`) instead of
    /// what `#[pyro(import = "...")]` gave, for a macro that knows where it
    /// expands.
    pub fn with_krate(mut self, krate: proc_macro2::TokenStream) -> Self {
        self.krate = krate;
        self
    }

    /// The struct's name.
    pub fn name(&self) -> &syn::Ident {
        &self.name
    }

    /// Each field's name and wire type, in declaration order.
    pub fn fields(&self) -> impl Iterator<Item = (&syn::Ident, &syn::Type)> {
        self.fields.iter().map(|f| (&f.ident, &f.ty))
    }

    /// Each field's name and the type it was written as, in declaration
    /// order: `foo: &str` where [`Self::fields`] gives `foo: String`.
    pub fn original_fields(&self) -> impl Iterator<Item = (&syn::Ident, &syn::Type)> {
        self.fields.iter().map(|f| (&f.ident, &f.original))
    }

    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    /// The struct's own word as a type: `u16` or `u32`.
    pub(crate) fn word_ty(&self) -> proc_macro2::TokenStream {
        match self.word {
            Word::U16 => quote::quote! { u16 },
            Word::U32 => quote::quote! { u32 },
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Field {
    pub ident: syn::Ident,
    /// The type on the wire, which every derive works from.
    pub ty: syn::Type,
    /// The type as it was written, before [`wire_type`]: `&str` for a
    /// `String` field built by [`StructInput::from_params`]. The same as
    /// `ty` on a derived struct, whose fields are always owned.
    pub original: syn::Type,
    pub resolved: ResolvedType,
    /// The field's `///` documentation, or empty.
    pub doc: String,
    /// `#[pyro(numeric)]`: the field is meant as numbers, not bytes, so a
    /// `Vec<u8>` in it is a `u8` list rather than `Bytes` (already applied to
    /// `resolved`). Only accepted on a numeric field.
    #[allow(dead_code)]
    pub numeric: bool,
    /// `#[pyro(allow(vec_bool))]`: silences the `Vec<bool>` advisory the
    /// `Writer` derive otherwise emits. Only accepted on a plain `Vec<bool>`.
    pub allow_vec_bool: bool,
}

impl StructInput {
    /// A non-generic struct with named fields (or none at all).
    pub fn parse(input: &DeriveInput) -> Result<Self> {
        if input
            .generics
            .params
            .iter()
            .any(|p| !matches!(p, syn::GenericParam::Lifetime(_)))
        {
            return Err(Error::new_spanned(
                &input.generics,
                "#[derive(Layout, Writer, Reader)] does not support generic type or const parameters yet",
            ));
        }
        if !input.generics.params.is_empty() {
            return Err(Error::new_spanned(
                &input.generics,
                "#[derive(Layout, Writer, Reader)] does not support lifetime parameters — every field must be owned",
            ));
        }

        let named: Vec<&syn::Field> = match &input.data {
            Data::Struct(s) => match &s.fields {
                Fields::Named(named) => named.named.iter().collect(),
                Fields::Unit => Vec::new(),
                Fields::Unnamed(_) => {
                    return Err(Error::new_spanned(
                        &input.ident,
                        "#[derive(Layout, Writer, Reader)] requires a struct with named fields",
                    ));
                }
            },
            _ => {
                return Err(Error::new_spanned(
                    &input.ident,
                    "#[derive(Layout, Writer, Reader)] can only be applied to structs",
                ));
            }
        };

        let mut fields = Vec::new();
        for field in named {
            // `#[pyro(numeric)]` and `#[pyro(allow(vec_bool))]`, parsed
            // together since they share the namespace.
            let mut numeric = false;
            let mut allow_vec_bool = false;
            for attr in &field.attrs {
                if !attr.path().is_ident("pyro") {
                    continue;
                }
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("numeric") {
                        numeric = true;
                        Ok(())
                    } else if meta.path.is_ident("allow") {
                        meta.parse_nested_meta(|inner| {
                            if inner.path.is_ident("vec_bool") {
                                allow_vec_bool = true;
                                Ok(())
                            } else {
                                Err(inner.error(
                                    "unknown #[pyro(allow(...))] option, expected `vec_bool`",
                                ))
                            }
                        })
                    } else {
                        Err(meta.error(
                            "unknown #[pyro(...)] option, expected `numeric` or `allow(vec_bool)`",
                        ))
                    }
                })?;
            }
            let resolved = type_map::resolve_type(&field.ty, numeric)?;
            let is_numeric = matches!(
                resolved,
                ResolvedType::Leaf { leaf: Leaf::Prim(p), .. } if p != Prim::Bool && p != Prim::Time
            );
            if numeric && !is_numeric {
                return Err(Error::new_spanned(
                    &field.ty,
                    "#[pyro(numeric)] only applies to a numeric field (`u8`, `u32`, `Vec<i64>`, `Vec<Vec<f64>>`, ...)",
                ));
            }
            if allow_vec_bool && !is_vec_bool(&resolved) {
                return Err(Error::new_spanned(
                    &field.ty,
                    "#[pyro(allow(vec_bool))] only applies to a `Vec<bool>` field, at any list depth",
                ));
            }

            fields.push(Field {
                ident: field.ident.clone().expect("named field"),
                ty: field.ty.clone(),
                original: field.ty.clone(),
                resolved,
                doc: parse_doc(&field.attrs),
                numeric,
                allow_vec_bool,
            });
        }

        Ok(StructInput {
            name: input.ident.clone(),
            krate: parse_import(&input.attrs)?,
            vis: input.vis.clone(),
            doc: parse_doc(&input.attrs),
            fields,
            word: parse_word(&input.attrs)?,
        })
    }
}

impl StructInput {
    /// A struct built from a function's parameters rather than one the user
    /// wrote: each borrowed parameter becomes its owned [`wire_type`], and
    /// keeps the borrowed one as its original type. `krate` is the path the
    /// generated code reaches `pyroformat` through, e.g. `::pyroduct`.
    pub fn from_params(
        name: syn::Ident,
        krate: proc_macro2::TokenStream,
        params: &[(syn::Ident, syn::Type)],
    ) -> Result<Self> {
        let fields = params
            .iter()
            .map(|(n, t)| wire_type(t).map(|t| quote::quote!(pub #n: #t)))
            .collect::<Result<Vec<_>>>()?;
        let input: DeriveInput = syn::parse_quote! {
            struct #name {
                #(#fields),*
            }
        };
        let mut input = Self::parse(&input)?.with_krate(krate);
        for (field, (_, original)) in input.fields.iter_mut().zip(params) {
            field.original = original.clone();
        }
        Ok(input)
    }
}

/// The owned type a borrowed one travels as: `&str` → `String`,
/// `&[T]` → `Vec<T>`, `&T` → `T`. Anything else is already owned.
pub fn wire_type(ty: &syn::Type) -> Result<syn::Type> {
    match ty {
        syn::Type::Reference(r) => {
            if r.mutability.is_some() {
                return Err(Error::new_spanned(
                    r,
                    "a `&mut` parameter can't be sent over the wire",
                ));
            }
            Ok(match &*r.elem {
                syn::Type::Path(p) if p.qself.is_none() && p.path.is_ident("str") => {
                    syn::parse_quote!(String)
                }
                syn::Type::Slice(slice) => {
                    let elem = &slice.elem;
                    syn::parse_quote!(Vec<#elem>)
                }
                elem => elem.clone(),
            })
        }
        ty => Ok(ty.clone()),
    }
}

/// `#[pyro(Word = u16)]` on the struct: the word its structure (topology
/// tiers, fixed-region pointers) is written in. `u16` when absent.
fn parse_word(attrs: &[syn::Attribute]) -> Result<Word> {
    let mut word: Option<Word> = None;
    for attr in attrs {
        if !attr.path().is_ident("pyro") {
            continue;
        }
        // Only `Word` is read here, as `parse_import` reads only `import`.
        let metas = attr.parse_args_with(
            syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
        )?;
        for meta in metas {
            let syn::Meta::NameValue(nv) = meta else {
                continue;
            };
            if !nv.path.is_ident("Word") {
                continue;
            }
            let parsed = match type_map::ungroup(&nv.value) {
                syn::Expr::Path(p) if p.path.is_ident("u16") => Word::U16,
                syn::Expr::Path(p) if p.path.is_ident("u32") => Word::U32,
                _ => {
                    return Err(Error::new_spanned(
                        &nv.value,
                        "#[pyro(Word = ...)] takes `u16` or `u32`",
                    ));
                }
            };
            if word.is_some() {
                return Err(Error::new_spanned(
                    &nv.path,
                    "#[pyro(Word = ...)] is set more than once",
                ));
            }
            word = Some(parsed);
        }
    }
    Ok(word.unwrap_or(Word::U16))
}

/// `#[pyro(import = "pyroduct")]` on the struct: generated code reaches
/// `pyroformat` as `::pyroduct` instead. A path (`"pyroduct::format"`) works
/// too; it is always taken from the crate root.
fn parse_import(attrs: &[syn::Attribute]) -> Result<proc_macro2::TokenStream> {
    let mut krate: Option<syn::Path> = None;
    for attr in attrs {
        if !attr.path().is_ident("pyro") {
            continue;
        }
        // Only `import` is read here; any other struct-level option is left
        // alone, as it was before `import` existed.
        let metas = attr.parse_args_with(
            syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
        )?;
        for meta in metas {
            let syn::Meta::NameValue(nv) = meta else {
                continue;
            };
            if !nv.path.is_ident("import") {
                continue;
            }
            let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(lit),
                ..
            }) = &nv.value
            else {
                return Err(Error::new_spanned(
                    &nv.value,
                    "#[pyro(import = \"...\")] takes a string, e.g. `import = \"pyroduct\"`",
                ));
            };
            let mut path: syn::Path = lit.parse()?;
            path.leading_colon = Some(Default::default());
            krate = Some(path);
        }
    }
    Ok(match krate {
        Some(path) => quote::quote! { #path },
        None => quote::quote! { ::pyroformat },
    })
}

/// `Vec<bool>`, at any list depth (`Vec<Vec<bool>>`, ..): the shapes the
/// `Writer` derive's `BitVec` advisory is for.
pub(crate) fn is_vec_bool(resolved: &ResolvedType) -> bool {
    matches!(
        resolved,
        ResolvedType::Leaf {
            leaf: Leaf::Prim(Prim::Bool),
            vec_depth: 1..
        }
    )
}

/// A `///` comment as one string: one line per `#[doc]`, the leading space
/// rustdoc adds stripped, blank lines at either end dropped.
fn parse_doc(attrs: &[syn::Attribute]) -> String {
    let mut lines = Vec::new();
    for attr in attrs {
        if !attr.path().is_ident("doc") {
            continue;
        }
        if let syn::Meta::NameValue(nv) = &attr.meta
            && let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(s),
                ..
            }) = &nv.value
        {
            let line = s.value();
            lines.push(line.strip_prefix(' ').unwrap_or(&line).to_string());
        }
    }
    while lines.first().is_some_and(|l: &String| l.trim().is_empty()) {
        lines.remove(0);
    }
    while lines.last().is_some_and(|l: &String| l.trim().is_empty()) {
        lines.pop();
    }
    lines.join("\n")
}
