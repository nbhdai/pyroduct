//! capability! - Unified capability export macro
//!
//! This macro acts as the glue layer. It parses the DSL, delegates to specific
//! handlers (Client, Server, Function, Trait), and assembles the final
//! Module (WASM) and Host (FFI) code blocks.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{
    Attribute, Ident, Item, ItemFn, ItemImpl, ItemStruct, ItemTrait, LitStr, Result, Token, parse2,
};

use crate::capability::CapabilityDefTrait;
use crate::capability_client::CapClient;
use crate::capability_ffi::CapabilityFuncFFI;
use crate::capability_function::CapFn;
use crate::capability_server::CapServer;
use crate::utils::{has_attr, remove_attr};

/// Parsed content from the capability! macro
pub struct CapabilityModule {
    /// Environment/module name (e.g., "http_client")
    pub env: Ident,
    /// env string literal for usage in code
    pub env_str: String,

    // Handlers
    pub clients: Vec<CapClient>,
    pub servers: Vec<CapServer>,
    pub traits: Vec<CapabilityDefTrait>,
    
    // We store the original ItemImpl to emit it unchanged,
    // alongside the CapabilityDefTrait derived from it for FFI generation.
    pub impls: Vec<(ItemImpl, CapabilityDefTrait)>,
    
    pub functions: Vec<CapFn>,

    // Pass-through items
    pub other_items: Vec<Item>,
}



impl Parse for CapabilityModule {
    fn parse(input: ParseStream) -> Result<Self> {
        // 1. Parse Metadata: env = "..."
        let _env_key: Ident = input.parse()?; // "env"
        let _: Token![=] = input.parse()?;
        let env_lit: LitStr = input.parse()?;
        let env_str = env_lit.value();
        let env = format_ident!("{}", env_str);

        if input.peek(Token![,]) {
            let _: Token![,] = input.parse()?;
        }

        let mut module = CapabilityModule {
            env,
            env_str,
            clients: Vec::new(),
            servers: Vec::new(),
            traits: Vec::new(),
            impls: Vec::new(),
            functions: Vec::new(),
            other_items: Vec::new(),
        };

        // 2. Parse Items and delegate to helpers
        while !input.is_empty() {
            let item: Item = input.parse()?;

            match item {
                // #[capability_client]
                Item::Struct(mut s) if has_attr(&s.attrs, "capability_client") => {
                    let attr = remove_attr(&mut s.attrs, "capability_client");
                    module.clients.push(CapClient::new(attr, s)?);
                }
                // #[capability_server]
                Item::Struct(mut s) if has_attr(&s.attrs, "capability_server") => {
                    let attr = remove_attr(&mut s.attrs, "capability_server");
                    module.servers.push(CapServer::new(attr, s)?);
                }
                // #[capability] trait
                Item::Trait(mut t) if has_attr(&t.attrs, "capability") => {
                    let _ = remove_attr(&mut t.attrs, "capability");
                    module
                        .traits
                        .push(CapabilityDefTrait::from_trait(t)?);
                }
                // #[capability_function]
                Item::Fn(mut f) if has_attr(&f.attrs, "capability_function") => {
                    let _ = remove_attr(&mut f.attrs, "capability_function");
                    module.functions.push(CapFn::new(f)?);
                }
                // impl Trait for Server (Capability Implementation)
                Item::Impl(item_impl) if item_impl.trait_.is_some() => {
                    // 1. Parse the definition for FFI generation
                    let cap_def = CapabilityDefTrait::from_impl(&item_impl)?;
                    // 2. Store BOTH the original item and the definition
                    module.impls.push((item_impl, cap_def));
                }
                // Pass-through
                other => module.other_items.push(other),
            }
        }

        Ok(module)
    }
}
