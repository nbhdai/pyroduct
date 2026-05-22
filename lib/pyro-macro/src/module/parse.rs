use proc_macro2;
use syn::{
    Expr, Ident, Meta, Result, Token,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};

/// The three supported output patterns
pub enum OutputSpec {
    /// Single field: `output = "field_name"`
    SingleField(Ident),
    /// Tuple fields: `output = (field1, field2, ...)`  
    TupleFields(Vec<Ident>),
    /// Existing struct: `output = StructName`
    Struct,
}

/// Parsed attributes for #[module(...)]
pub struct ModuleAttrs {
    pub session: bool,
    pub output: OutputSpec,
}

impl ModuleAttrs {
    fn from_metas(metas: Punctuated<Meta, Token![,]>) -> Result<Self> {
        let mut session = false;
        let mut output = None;

        for meta in metas {
            match meta {
                Meta::Path(path) if path.is_ident("session") => {
                    session = true;
                }
                Meta::NameValue(nv) if nv.path.is_ident("output") => {
                    output = Some(parse_output_spec(&nv.value)?);
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        meta,
                        "Unexpected attribute. Expected 'session' or 'output = ...'",
                    ));
                }
            }
        }

        let output = output.ok_or_else(|| {
            syn::Error::new(proc_macro2::Span::call_site(), "Missing `output` attribute")
        })?;

        Ok(ModuleAttrs { session, output })
    }
}

fn parse_output_spec(expr: &Expr) -> Result<OutputSpec> {
    match expr {
        Expr::Tuple(tuple) => {
            let mut fields = Vec::new();
            for e in &tuple.elems {
                if let Expr::Path(path) = e {
                    if let Some(ident) = path.path.get_ident() {
                        fields.push(ident.clone());
                    } else {
                        return Err(syn::Error::new_spanned(
                            path,
                            "Expected identifier in tuple",
                        ));
                    }
                } else {
                    return Err(syn::Error::new_spanned(e, "Expected identifier in tuple"));
                }
            }
            Ok(OutputSpec::TupleFields(fields))
        }
        Expr::Path(path) => {
            if let Some(ident) = path.path.get_ident() {
                let name_str = ident.to_string();
                if name_str
                    .chars()
                    .next()
                    .map(|c| c.is_uppercase())
                    .unwrap_or(false)
                {
                    Ok(OutputSpec::Struct)
                } else {
                    Ok(OutputSpec::SingleField(ident.clone()))
                }
            } else {
                Err(syn::Error::new_spanned(
                    path,
                    "Expected identifier for output",
                ))
            }
        }
        _ => Err(syn::Error::new_spanned(
            expr,
            "Expected identifier or tuple of identifiers for output",
        )),
    }
}

impl Parse for ModuleAttrs {
    fn parse(input: ParseStream) -> Result<Self> {
        let metas = Punctuated::<Meta, Token![,]>::parse_terminated(input)?;
        Self::from_metas(metas)
    }
}
