use proc_macro::TokenStream;

use module_core;
use syn::{parse2, ItemFn};

    

#[proc_macro_attribute]
pub fn module(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attrs: module_core::ModuleAttrs = match parse2(attr.into()) {
        Ok(attrs) => attrs,
        Err(error) => return error.to_compile_error().into(),
    };
    let input_fn: ItemFn  = match parse2(item.into()) {
        Ok(attrs) => attrs,
        Err(error) => return error.to_compile_error().into(),
    };

    module_core::expand(attrs, input_fn)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}
