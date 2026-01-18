use quote::format_ident;
use syn::Ident;

pub mod client;
pub mod constructors;
pub mod state;
pub mod methods;
pub mod export;
pub mod definition;

pub struct ClassIdent {
    pub trait_tn: Ident,
    pub state_tn: Ident,
    pub client_tn: Ident,
    pub error_tn: Option<Ident>,
}

impl ClassIdent {
    pub fn function_path(&self, function_name: &Ident) -> Ident {
        format_ident!("__{}__{}__{}", self.trait_tn.to_string(), )
    }
}