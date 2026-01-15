use pyroduct::{capability, capability_server};
use serde::Deserialize;
pub trait Configured {
    fn call(&mut self);
}
pub struct MyConfig {
    limit: u32,
}
#[doc(hidden)]
#[allow(
    non_upper_case_globals,
    unused_attributes,
    unused_qualifications,
    clippy::absolute_paths,
)]
const _: () = {
    #[allow(unused_extern_crates, clippy::useless_attribute)]
    extern crate serde as _serde;
    #[automatically_derived]
    impl<'de> _serde::Deserialize<'de> for MyConfig {
        fn deserialize<__D>(
            __deserializer: __D,
        ) -> _serde::__private228::Result<Self, __D::Error>
        where
            __D: _serde::Deserializer<'de>,
        {
            #[allow(non_camel_case_types)]
            #[doc(hidden)]
            enum __Field {
                __field0,
                __ignore,
            }
            #[doc(hidden)]
            struct __FieldVisitor;
            #[automatically_derived]
            impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
                type Value = __Field;
                fn expecting(
                    &self,
                    __formatter: &mut _serde::__private228::Formatter,
                ) -> _serde::__private228::fmt::Result {
                    _serde::__private228::Formatter::write_str(
                        __formatter,
                        "field identifier",
                    )
                }
                fn visit_u64<__E>(
                    self,
                    __value: u64,
                ) -> _serde::__private228::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        0u64 => _serde::__private228::Ok(__Field::__field0),
                        _ => _serde::__private228::Ok(__Field::__ignore),
                    }
                }
                fn visit_str<__E>(
                    self,
                    __value: &str,
                ) -> _serde::__private228::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        "limit" => _serde::__private228::Ok(__Field::__field0),
                        _ => _serde::__private228::Ok(__Field::__ignore),
                    }
                }
                fn visit_bytes<__E>(
                    self,
                    __value: &[u8],
                ) -> _serde::__private228::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        b"limit" => _serde::__private228::Ok(__Field::__field0),
                        _ => _serde::__private228::Ok(__Field::__ignore),
                    }
                }
            }
            #[automatically_derived]
            impl<'de> _serde::Deserialize<'de> for __Field {
                #[inline]
                fn deserialize<__D>(
                    __deserializer: __D,
                ) -> _serde::__private228::Result<Self, __D::Error>
                where
                    __D: _serde::Deserializer<'de>,
                {
                    _serde::Deserializer::deserialize_identifier(
                        __deserializer,
                        __FieldVisitor,
                    )
                }
            }
            #[doc(hidden)]
            struct __Visitor<'de> {
                marker: _serde::__private228::PhantomData<MyConfig>,
                lifetime: _serde::__private228::PhantomData<&'de ()>,
            }
            #[automatically_derived]
            impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                type Value = MyConfig;
                fn expecting(
                    &self,
                    __formatter: &mut _serde::__private228::Formatter,
                ) -> _serde::__private228::fmt::Result {
                    _serde::__private228::Formatter::write_str(
                        __formatter,
                        "struct MyConfig",
                    )
                }
                #[inline]
                fn visit_seq<__A>(
                    self,
                    mut __seq: __A,
                ) -> _serde::__private228::Result<Self::Value, __A::Error>
                where
                    __A: _serde::de::SeqAccess<'de>,
                {
                    let __field0 = match _serde::de::SeqAccess::next_element::<
                        u32,
                    >(&mut __seq)? {
                        _serde::__private228::Some(__value) => __value,
                        _serde::__private228::None => {
                            return _serde::__private228::Err(
                                _serde::de::Error::invalid_length(
                                    0usize,
                                    &"struct MyConfig with 1 element",
                                ),
                            );
                        }
                    };
                    _serde::__private228::Ok(MyConfig { limit: __field0 })
                }
                #[inline]
                fn visit_map<__A>(
                    self,
                    mut __map: __A,
                ) -> _serde::__private228::Result<Self::Value, __A::Error>
                where
                    __A: _serde::de::MapAccess<'de>,
                {
                    let mut __field0: _serde::__private228::Option<u32> = _serde::__private228::None;
                    while let _serde::__private228::Some(__key) = _serde::de::MapAccess::next_key::<
                        __Field,
                    >(&mut __map)? {
                        match __key {
                            __Field::__field0 => {
                                if _serde::__private228::Option::is_some(&__field0) {
                                    return _serde::__private228::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field("limit"),
                                    );
                                }
                                __field0 = _serde::__private228::Some(
                                    _serde::de::MapAccess::next_value::<u32>(&mut __map)?,
                                );
                            }
                            _ => {
                                let _ = _serde::de::MapAccess::next_value::<
                                    _serde::de::IgnoredAny,
                                >(&mut __map)?;
                            }
                        }
                    }
                    let __field0 = match __field0 {
                        _serde::__private228::Some(__field0) => __field0,
                        _serde::__private228::None => {
                            _serde::__private228::de::missing_field("limit")?
                        }
                    };
                    _serde::__private228::Ok(MyConfig { limit: __field0 })
                }
            }
            #[doc(hidden)]
            const FIELDS: &'static [&'static str] = &["limit"];
            _serde::Deserializer::deserialize_struct(
                __deserializer,
                "MyConfig",
                FIELDS,
                __Visitor {
                    marker: _serde::__private228::PhantomData::<MyConfig>,
                    lifetime: _serde::__private228::PhantomData,
                },
            )
        }
    }
};
pub struct ConfiguredServer {
    limit: u32,
}
pub trait ConfiguredServerInit {
    fn new() -> Self;
    fn with_config(config: MyConfig) -> Self;
    fn reset(&mut self);
}
pub mod __configured_server_ffi {
    use super::*;
    #[unsafe(no_mangle)]
    pub extern "C" fn plugin_init(
        config_ptr: *const u8,
        config_len: usize,
    ) -> ::pyroduct::capability_host::FfiInitResult {
        unsafe {
            ::pyroduct::capability::safe_lifecycle::execute_safe_init::<
                MyConfig,
                ConfiguredServer,
                _,
            >(
                config_ptr,
                config_len,
                |config| <ConfiguredServer as ConfiguredServerInit>::with_config(config),
            )
        }
    }
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn plugin_drop(state: *mut std::ffi::c_void) {
        if !state.is_null() {
            drop(unsafe { Box::from_raw(state as *mut ConfiguredServer) });
        }
    }
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn plugin_reset(
        state: *mut std::ffi::c_void,
    ) -> ::pyroduct::capability_host::FfiResult {
        ::pyroduct::capability::safe_lifecycle::execute_safe_reset::<
            ConfiguredServer,
            _,
        >(state, |state| <ConfiguredServer as ConfiguredServerInit>::reset(state))
    }
    pub const INIT_FN: ::pyroduct::capability_host::ffi::PluginInitFn = ::pyroduct::capability_host::ffi::PluginInitFn::Sync(
        plugin_init,
    );
    pub const DROP_FN: ::pyroduct::capability_host::ffi::PluginDropFn = ::pyroduct::capability_host::ffi::PluginDropFn::Sync(
        plugin_drop,
    );
    pub const RESET_FN: ::pyroduct::capability_host::ffi::PluginResetFn = ::pyroduct::capability_host::ffi::PluginResetFn::Sync(
        plugin_reset,
    );
}
impl ConfiguredServerInit for ConfiguredServer {
    fn new() -> Self {
        Self { limit: 10 }
    }
    fn with_config(config: MyConfig) -> Self {
        Self { limit: config.limit }
    }
    fn reset(&mut self) {}
}
