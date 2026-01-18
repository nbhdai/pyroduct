//! capability! - Unified capability export macro
//!
//! This macro acts as the glue layer. It parses the DSL, resolves relationships
//! between Clients, Servers, Traits, and Implementations, and assembles the
//! final Module (WASM) and Host (FFI) code blocks.

use std::collections::HashMap;

use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{
    Error, Ident, Item, ItemImpl, ItemTrait, LitStr, Result, Token,
};

use crate::classes::{state::CapServer, definition::CapabilityDefTrait, client::CapClient, export::CapabilityService};
use crate::ffi::CapabilityFuncFFI;
use crate::function::CapFn;
use crate::utils::{extract_ident_ignoring_ref, extract_simple_trait_ident, has_attr, remove_attr};



/// Parsed content from the capability! macro
pub struct CapabilityModule {
    /// Environment/module name (e.g., "http_client")
    pub env: Ident,
    /// env string literal for usage in code
    pub env_str: String,

    // Handlers
    pub services: Vec<CapabilityService>,
    pub functions: Vec<CapFn>,

    // Pass-through items (includes unlinked items or standalone definitions)
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

        // 2. Parse all remaining items into a buffer first
        let mut all_items: Vec<Item> = Vec::new();
        while !input.is_empty() {
            all_items.push(input.parse()?);
        }

        // Storage
        let mut parsed_impls: Vec<ItemImpl> = Vec::new();
        let mut parsed_functions: Vec<CapFn> = Vec::new();
        let mut parsed_traits: HashMap<Ident, ItemTrait> = HashMap::new();
        
        // ==============================================================================
        // Pass 1: Implementations and Functions
        // ==============================================================================
        let mut remaining_items = Vec::new();
        
        for item in all_items {
            match item {
                // impl Trait for Server
                Item::Impl(mut i) if has_attr(&i.attrs, "capability") => {
                    let _ = remove_attr(&mut i.attrs, "capability");
                    parsed_impls.push(i);
                }
                // #[capability_function]
                Item::Fn(mut f) if has_attr(&f.attrs, "capability") => {
                    let _ = remove_attr(&mut f.attrs, "capability");
                    parsed_functions.push(CapFn::new(f)?);
                }
                // #[capability] trait
                Item::Trait(mut t) if has_attr(&t.attrs, "capability") => {
                    let _ = remove_attr(&mut t.attrs, "capability");
                    let name = t.ident.clone();
                    parsed_traits.insert(name, t);
                }
                // Determine other items for Pass 2
                _ => remaining_items.push(item),
            }
        }

        // ==============================================================================
        // Pass 2: Servers, Clients, and Implementations
        // ==============================================================================
        let mut parsed_servers: HashMap<Ident, CapServer> = HashMap::new();
        let mut parsed_clients: HashMap<Ident, CapClient> = HashMap::new();
        let mut other_items = Vec::new();
        for item in remaining_items {
            match item {
                // #[capability_client]
                Item::Struct(mut s) if has_attr(&s.attrs, "capability_client") => {
                    remove_attr(&mut s.attrs, "capability_client");
                    let client = CapClient::new(s)?;
                    parsed_clients.insert(client.ident.clone(), client);
                }
                // #[capability_server]
                Item::Struct(mut s) if has_attr(&s.attrs, "capability_server") => {
                    let attr = remove_attr(&mut s.attrs, "capability_server");
                    let server = CapServer::new(attr, s)?;
                    parsed_servers.insert(server.struct_name.clone(), server);
                }
                // Pass-through
                other => other_items.push(other),
            }
        }

        // ==============================================================================
        // Resolution: Link Services
        // ==============================================================================
        let mut services = Vec::new();

        for impl_item in parsed_impls {
            // A. Identify Server
            let struct_name = extract_ident_ignoring_ref(&*impl_item.self_ty).ok_or({
                Error::new_spanned(&impl_item.self_ty, "Unable to parse the server, should be a simple ident")
            })?;
            let trait_name = extract_simple_trait_ident(&impl_item)?;
            let trait_def = if let Some(capability_trait) = parsed_traits.remove(&trait_name) {
                CapabilityDefTrait::from_trait(capability_trait, struct_name.clone())?
            } else {
                return Err(Error::new_spanned(impl_item, "Cannot parse the associated server"));
            };


            let server = if let Some(server) = parsed_servers.remove(&struct_name) {
                server
            } else {
                // Impl exists but no matching #[capability_server] found in this block.
                return Err(Error::new_spanned(impl_item, "Cannot find the associated server"));
            };

            // Resolve Client (based on type defined in Trait)
            let client_def = if let Some(c) = parsed_clients.remove(&trait_def.client_name) {
                c
            } else {
                return Err(Error::new_spanned(impl_item, "Cannot find the associated client"));
            };

            services.push(CapabilityService {
                struct_def: server,
                trait_def,
                orig_impl: impl_item,
                client: client_def,
            });

            
        }

        // ==============================================================================
        // Cleanup: Re-add unused definitions
        // ==============================================================================
        // Any servers or clients that were defined but not linked to an implementation
        // are returned to the `other_items` list as standard structs (attributes removed).
        
        // for (_, server) in parsed_servers {
        //     other_items.push(Item::Struct(server.input));
        // }

        // for (_, client) in parsed_clients {
        //     other_items.push(Item::Struct(client.input));
        // }

        Ok(CapabilityModule {
            env,
            env_str,
            services,
            functions: parsed_functions,
            other_items,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;
    use syn::parse2;

    // Helper: Counts how many structs with a specific name exist in `other_items`.
    // This helps verify if unused items were correctly returned to the pool.
    fn count_structs_in_other(items: &[Item], name: &str) -> usize {
        items
            .iter()
            .filter(|i| {
                if let Item::Struct(s) = i {
                    s.ident == name
                } else {
                    false
                }
            })
            .count()
    }

    #[test]
    fn test_full_resolution_happy_path() {
        let input = quote! {
            env = "test_env",

            #[capability_client]
            struct MyClient { val: u32 }

            #[capability_server(config = MyServer)]
            struct MyServer { internal: i32 }

            #[capability]
            trait MyTrait {
                type Client = MyClient;
                fn new_client(val: u32) -> MyClient {
                    MyClient {
                        val
                    }
                }
                fn do_thing() -> bool;
            }

            // Link them all together
            #[capability]
            impl MyTrait for MyServer {
                type Client = MyClient;
                fn new_client(&self, c: &MyClient) -> Result<(), ()> { Ok(()) }
                fn do_thing(&self, c: &MyClient) -> Result<bool, ()> { Ok(true) }
            }
        };

        let module: CapabilityModule = parse2(input).expect("Failed to parse module");

        assert_eq!(module.env.to_string(), "test_env");
        assert_eq!(module.services.len(), 1, "Should resolve exactly one service");
        assert_eq!(module.functions.len(), 0);

        // Check Service Resolution details
        let service = &module.services[0];
        assert_eq!(service.struct_def.struct_name.to_string(), "MyServer");
        assert_eq!(service.trait_def.trait_name.to_string(), "MyTrait");

        // Verify used items are removed from other_items
        assert_eq!(count_structs_in_other(&module.other_items, "MyClient"), 0);
        assert_eq!(count_structs_in_other(&module.other_items, "MyServer"), 0);
    }

    #[test]
    fn test_unused_cleanup_pass() {
        // This tests the specific logic: "strip clients and servers... put them back if not used"
        let input = quote! {
            env = "cleanup_test",

            struct UsedConfig { a: u32 }

            // --- Used Pair ---
            #[capability_client]
            struct UsedClient { a: u32 }

            #[capability_server(config = UsedConfig)]
            struct UsedServer { b: i32 }

            // --- Unused Pair ---
            struct UnusedConfig { a: i32 }
            #[capability_client]
            struct UnusedClient { c: u32 }
            #[capability_server(config = UnusedConfig)]
            struct UnusedServer { d: i32 }

            #[capability]
            trait MyTrait {
                type Client = UsedClient;
                fn new_client(a: u32) -> UsedClient {
                    UsedClient {
                        a
                    }
                }
                fn op();
            }

            #[capability]
            impl MyTrait for UsedServer {
                type Client = UsedClient;
                fn new_client(&self, c: &UsedClient) { Ok(()) }
                fn op(&self, c: &UsedClient) -> Result<(), ()> { Ok(()) }
            }
        };

        let module: CapabilityModule = parse2(input).expect("Failed to parse");

        // 1. Verify the used service was resolved
        assert_eq!(module.services.len(), 1);
        let service = &module.services[0];
        assert_eq!(service.struct_def.struct_name.to_string(), "UsedServer");

        // 2. Verify UNUSED items were returned to other_items
        assert_eq!(
            count_structs_in_other(&module.other_items, "UnusedClient"),
            1,
            "UnusedClient should appear in other_items"
        );
        assert_eq!(
            count_structs_in_other(&module.other_items, "UnusedServer"),
            1,
            "UnusedServer should appear in other_items"
        );

        // 3. Verify USED items are NOT in other_items (they are inside the Service object)
        assert_eq!(
            count_structs_in_other(&module.other_items, "UsedClient"),
            0,
            "UsedClient should NOT be in other_items"
        );
        assert_eq!(
            count_structs_in_other(&module.other_items, "UsedServer"),
            0,
            "UsedServer should NOT be in other_items"
        );
    }

    #[test]
    fn test_standalone_functions_pass_1() {
        let input = quote! {
            env = "func_env",
            
            #[capability]
            fn standalone_one() {}

            #[capability]
            fn standalone_two(x: u32) -> u32 { x }
        };

        let module: CapabilityModule = parse2(input).expect("Failed to parse");
        assert_eq!(module.functions.len(), 2);
        assert_eq!(module.services.len(), 0);
    }

    #[test]
    fn test_passthrough_items() {
        let input = quote! {
            env = "test",
            struct PlainStruct;
            fn plain_fn() {}
        };
        let module: CapabilityModule = parse2(input).expect("Failed to parse");

        assert_eq!(module.services.len(), 0);
        assert_eq!(module.functions.len(), 0);
        // Expect 2 items: PlainStruct and plain_fn
        assert_eq!(module.other_items.len(), 2);
    }
}