use syn::{Ident, Result, Token, parenthesized, parse::{Parse, ParseStream}, punctuated::Punctuated};


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
    pub output: OutputSpec,
}

impl Parse for ModuleAttrs {
    fn parse(input: ParseStream) -> Result<Self> {
        // Expect: output = ...
        let ident: Ident = input.parse()?;
        if ident != "output" {
            return Err(syn::Error::new(ident.span(), "Expected `output = ...`"));
        }

        input.parse::<Token![=]>()?;

        // Determine which pattern based on what follows
        let output = if input.peek(syn::token::Paren) {
            // Pattern 2: (field1, field2, ...)
            let content;
            parenthesized!(content in input);
            let fields: Punctuated<Ident, Token![,]> =
                content.parse_terminated(Ident::parse, Token![,])?;
            OutputSpec::TupleFields(fields.into_iter().collect())
        } else {
            // Could be Pattern 1 (lowercase field) or Pattern 3 (PascalCase struct)
            let name: Ident = input.parse()?;
            let name_str = name.to_string();

            // Heuristic: PascalCase = struct, snake_case/lowercase = field
            if name_str
                .chars()
                .next()
                .map(|c| c.is_uppercase())
                .unwrap_or(false)
            {
                OutputSpec::Struct
            } else {
                OutputSpec::SingleField(name)
            }
        };

        Ok(ModuleAttrs { output })
    }
}