use syn::{
    Ident, Result, Token, parenthesized,
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

impl Parse for ModuleAttrs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut session = false;

        // Check for optional 'session' flag at the start
        if input.peek(Ident) {
            let lookahead: Ident = input.parse()?;
            if lookahead == "session" {
                session = true;
                // Expect comma after session
                input.parse::<Token![,]>()?;
            } else if lookahead != "output" {
                // Unexpected identifier
                return Err(syn::Error::new(
                    lookahead.span(),
                    "Unexpected attribute. Expected 'session' or 'output = ...'",
                ));
            }
            // If lookahead == "output", we fall through to parse output = ...
        }

        // Expect: output = (consumed if not already done above)
        if !session {
            // We already consumed "output" when lookahead was "output", skip to "="
            input.parse::<Token![=]>()?;
        } else {
            // We need to parse "output" and then "="
            let ident: Ident = input.parse()?;
            if ident != "output" {
                return Err(syn::Error::new(
                    ident.span(),
                    "Expected `output = ...`",
                ));
            }
            input.parse::<Token![=]>()?;
        }

        // Now parse the output spec value
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

        Ok(ModuleAttrs { session, output })
    }
}
