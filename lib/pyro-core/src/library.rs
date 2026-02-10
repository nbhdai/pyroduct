use syn::{
    Ident, LitStr, Token,
    parse::{Parse, ParseStream},
};

pub struct LibraryArgs {
    pub meta: String,
    pub no_ffi: bool,
}

impl Parse for LibraryArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut meta = String::new();
        let mut no_ffi = false;

        while !input.is_empty() {
            if input.peek(LitStr) {
                let s: LitStr = input.parse()?;
                meta = s.value();
            } else if input.peek(Ident) {
                let id: Ident = input.parse()?;
                if id == "NoFFI" {
                    no_ffi = true;
                }
            }

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            } else {
                break;
            }
        }

        Ok(LibraryArgs { meta, no_ffi })
    }
}

/// Creates a function we can use to register the library identity
pub fn create_ident(
    import_location: syn::Path,
    meta: &str,
    no_ffi: bool,
) -> proc_macro2::TokenStream {
    let ffi_exports = if !no_ffi {
        quote::quote! {
            #[unsafe(no_mangle)]
            pub extern "C" fn library_json_ptr() -> *const u8 {
                LIBRARY_IDENTITY_DATA.as_ptr()
            }

            #[unsafe(no_mangle)]
            pub extern "C" fn library_json_len() -> usize {
                LIBRARY_IDENTITY_DATA.len()
            }
        }
    } else {
        quote::quote! {}
    };

    quote::quote! {
        #[derive(Debug, Clone, Copy)]
        pub struct Library;

        pub const LIBRARY_IDENTITY_DATA: &'static str = concat!(
            "{",
            "\"meta\": \"", #meta, "\",",
            "\"name\": \"", env!("CARGO_PKG_NAME"), "\",",
            "\"version\": \"", env!("CARGO_PKG_VERSION"), "\"",
            "}"
        );

        impl Library {
            pub const META: &'static str = #meta;
            pub const NAME: &'static str = env!("CARGO_PKG_NAME");
            pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");

            pub fn register_info() -> #import_location::captured::LibraryInfo<'static> {
                let info = #import_location::captured::LibraryInfo {
                    meta: ::std::borrow::Cow::Borrowed(Self::META),
                    name: ::std::borrow::Cow::Borrowed(Self::NAME),
                    version: ::std::borrow::Cow::Borrowed(Self::VERSION),
                };
                #import_location::captured::register_app_identity(info.clone(), LIBRARY_IDENTITY_DATA);

                info
            }
        }

        #ffi_exports
    }
}
