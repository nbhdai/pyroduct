//! FFI Export Table Generation for CapabilityService
//!
//! This module generates the `PluginExports` structure and associated
//! registration logic for a complete capability service.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{ItemImpl, Result};

use crate::classes::{client::CapClient, definition::CapabilityDefTrait, state::CapServer};

/// Represents a fully resolved service consisting of a Server, its Trait definition,
/// the User's Implementation, and the associated Client.
pub struct CapabilityService {
    pub struct_def: CapServer,
    pub trait_def: CapabilityDefTrait,
    pub orig_impl: ItemImpl,
    pub client: CapClient,
}

impl CapabilityService {
    /// Client
    pub fn client(&self) -> Result<TokenStream> {
        let client = self.client.expand();
        let client_methods = self.trait_def.generate_client_impl();
        
        Ok(quote! {
            #client
            #client_methods
        })
    }
    /// State and lifecycle
    pub fn state_and_lifecycle(&self) -> TokenStream {
        let state = &self.struct_def.input;
        let lifecycle = self.struct_def.generate_init_trait();
        
        quote! {
            #state
            #lifecycle
        }
    }
    /// Original Impl
    pub fn orig_impl(&self) -> &ItemImpl {
        &self.orig_impl
    }
    /// Generates the complete FFI export table for this service.
    ///
    /// This creates:
    /// 1. An array of `PluginExport` entries (one per method)
    /// 2. A `PluginExports` struct containing:
    ///    - Pointer to the export array
    ///    - Init function pointer
    ///    - Drop function pointer
    ///    - Reset function pointer
    ///    - Array length and capacity
    pub fn generate_ffi_exports(&self) -> TokenStream {
        let struct_name = &self.struct_def.struct_name;
        let trait_name = &self.trait_def.trait_name;
        
        // Get all capability FFIs (includes constructor + methods)
        let capability_ffis = self.trait_def.capability_ffis();
        
        // Generate init, drop, and reset function pointers
        let (init_ffi_func, init_export) = self.struct_def.generate_init_fn();
        let (drop_ffi_func, drop_export) = self.struct_def.generate_drop_fn();
        let (reset_ffi_func, reset_export) = self.struct_def.generate_reset_fn();
        
        // Generate all capability FFI functions
        let capability_ffi_funcs: Vec<_> = capability_ffis
            .iter()
            .map(|ffi| ffi.generate_capability_ffi())
            .collect();
        
        // Generate the PluginExport array entries
        let plugin_exports: Vec<_> = capability_ffis
            .iter()
            .map(|ffi| {
                let library = ffi.library.to_string();
                let fn_name = ffi.fn_name.to_string();
                let fn_ffi_name = &ffi.fn_ffi_name;
                
                let func_variant = if ffi.is_async {
                    quote! {
                        ::pyroduct::capability_host::ffi::PluginFunction::Async(#fn_ffi_name)
                    }
                } else {
                    quote! {
                        ::pyroduct::capability_host::ffi::PluginFunction::Sync(#fn_ffi_name)
                    }
                };
                
                quote! {
                    ::pyroduct::capability_host::ffi::PluginExport {
                        module: #library.as_ptr(),
                        module_len: #library.len(),
                        name: #fn_name.as_ptr(),
                        name_len: #fn_name.len(),
                        func: #func_variant,
                    }
                }
            })
            .collect();
        
        let num_exports = plugin_exports.len();
        
        // Generate the static export array name
        let exports_array_name = quote::format_ident!("__{}__{}__EXPORTS", 
            heck::AsSnakeCase(trait_name.to_string()).to_string().to_uppercase(),
            heck::AsSnakeCase(struct_name.to_string()).to_string().to_uppercase()
        );
        
        // Generate the PluginExports struct name
        let plugin_exports_name = quote::format_ident!("__{}__{}__PLUGIN_EXPORTS",
            heck::AsSnakeCase(trait_name.to_string()).to_string().to_uppercase(),
            heck::AsSnakeCase(struct_name.to_string()).to_string().to_uppercase()
        );
        
        quote! {
            // Generate all FFI functions
            #init_ffi_func
            #drop_ffi_func
            #reset_ffi_func
            
            #(#capability_ffi_funcs)*
            
            // Generate the static export array
            static #exports_array_name: [::pyroduct::capability_host::ffi::PluginExport; #num_exports] = [
                #(#plugin_exports),*
            ];
            
            // Generate the PluginExports struct
            pub static #plugin_exports_name: ::pyroduct::capability_host::ffi::PluginExports = {
                let mut exports = #exports_array_name;
                ::pyroduct::capability_host::ffi::PluginExports {
                    ptr: exports.as_mut_ptr(),
                    init: #init_export,
                    drop: #drop_export,
                    reset: #reset_export,
                    len: #num_exports,
                    cap: #num_exports,
                }
            };
        }
    }
}