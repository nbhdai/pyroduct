use serde::{Deserialize, Serialize};
pub struct SerialConfig {
    pub ports: Vec<String>,
}
#[automatically_derived]
impl ::core::clone::Clone for SerialConfig {
    #[inline]
    fn clone(&self) -> SerialConfig {
        SerialConfig {
            ports: ::core::clone::Clone::clone(&self.ports),
        }
    }
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
    impl _serde::Serialize for SerialConfig {
        fn serialize<__S>(
            &self,
            __serializer: __S,
        ) -> _serde::__private228::Result<__S::Ok, __S::Error>
        where
            __S: _serde::Serializer,
        {
            let mut __serde_state = _serde::Serializer::serialize_struct(
                __serializer,
                "SerialConfig",
                false as usize + 1,
            )?;
            _serde::ser::SerializeStruct::serialize_field(
                &mut __serde_state,
                "ports",
                &self.ports,
            )?;
            _serde::ser::SerializeStruct::end(__serde_state)
        }
    }
};
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
    impl<'de> _serde::Deserialize<'de> for SerialConfig {
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
                        "ports" => _serde::__private228::Ok(__Field::__field0),
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
                        b"ports" => _serde::__private228::Ok(__Field::__field0),
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
                marker: _serde::__private228::PhantomData<SerialConfig>,
                lifetime: _serde::__private228::PhantomData<&'de ()>,
            }
            #[automatically_derived]
            impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                type Value = SerialConfig;
                fn expecting(
                    &self,
                    __formatter: &mut _serde::__private228::Formatter,
                ) -> _serde::__private228::fmt::Result {
                    _serde::__private228::Formatter::write_str(
                        __formatter,
                        "struct SerialConfig",
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
                        Vec<String>,
                    >(&mut __seq)? {
                        _serde::__private228::Some(__value) => __value,
                        _serde::__private228::None => {
                            return _serde::__private228::Err(
                                _serde::de::Error::invalid_length(
                                    0usize,
                                    &"struct SerialConfig with 1 element",
                                ),
                            );
                        }
                    };
                    _serde::__private228::Ok(SerialConfig { ports: __field0 })
                }
                #[inline]
                fn visit_map<__A>(
                    self,
                    mut __map: __A,
                ) -> _serde::__private228::Result<Self::Value, __A::Error>
                where
                    __A: _serde::de::MapAccess<'de>,
                {
                    let mut __field0: _serde::__private228::Option<Vec<String>> = _serde::__private228::None;
                    while let _serde::__private228::Some(__key) = _serde::de::MapAccess::next_key::<
                        __Field,
                    >(&mut __map)? {
                        match __key {
                            __Field::__field0 => {
                                if _serde::__private228::Option::is_some(&__field0) {
                                    return _serde::__private228::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field("ports"),
                                    );
                                }
                                __field0 = _serde::__private228::Some(
                                    _serde::de::MapAccess::next_value::<
                                        Vec<String>,
                                    >(&mut __map)?,
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
                            _serde::__private228::de::missing_field("ports")?
                        }
                    };
                    _serde::__private228::Ok(SerialConfig { ports: __field0 })
                }
            }
            #[doc(hidden)]
            const FIELDS: &'static [&'static str] = &["ports"];
            _serde::Deserializer::deserialize_struct(
                __deserializer,
                "SerialConfig",
                FIELDS,
                __Visitor {
                    marker: _serde::__private228::PhantomData::<SerialConfig>,
                    lifetime: _serde::__private228::PhantomData,
                },
            )
        }
    }
};
pub struct SerialHandle {
    pub id: u64,
    #[rkyv(with = rkyv::with::Skip)]
    __config_buf: Vec<u8>,
}
#[automatically_derived]
impl ::core::clone::Clone for SerialHandle {
    #[inline]
    fn clone(&self) -> SerialHandle {
        SerialHandle {
            id: ::core::clone::Clone::clone(&self.id),
            __config_buf: ::core::clone::Clone::clone(&self.__config_buf),
        }
    }
}
#[automatically_derived]
///An archived [`SerialHandle`]
#[bytecheck(crate = ::rkyv::bytecheck)]
#[repr(C)]
pub struct ArchivedSerialHandle
where
    u64: ::rkyv::Archive,
    rkyv::with::Skip: ::rkyv::with::ArchiveWith<Vec<u8>>,
{
    ///The archived counterpart of [`SerialHandle::id`]
    pub id: <u64 as ::rkyv::Archive>::Archived,
    ///The archived counterpart of [`SerialHandle::__config_buf`]
    __config_buf: <rkyv::with::Skip as ::rkyv::with::ArchiveWith<Vec<u8>>>::Archived,
}
#[automatically_derived]
unsafe impl<
    __C: ::rkyv::bytecheck::rancor::Fallible + ?::core::marker::Sized,
> ::rkyv::bytecheck::CheckBytes<__C> for ArchivedSerialHandle
where
    u64: ::rkyv::Archive,
    rkyv::with::Skip: ::rkyv::with::ArchiveWith<Vec<u8>>,
    <__C as ::rkyv::bytecheck::rancor::Fallible>::Error: ::rkyv::bytecheck::rancor::Trace,
    <u64 as ::rkyv::Archive>::Archived: ::rkyv::bytecheck::CheckBytes<__C>,
    <rkyv::with::Skip as ::rkyv::with::ArchiveWith<
        Vec<u8>,
    >>::Archived: ::rkyv::bytecheck::CheckBytes<__C>,
{
    unsafe fn check_bytes(
        value: *const Self,
        context: &mut __C,
    ) -> ::core::result::Result<
        (),
        <__C as ::rkyv::bytecheck::rancor::Fallible>::Error,
    > {
        <<u64 as ::rkyv::Archive>::Archived as ::rkyv::bytecheck::CheckBytes<
            __C,
        >>::check_bytes(&raw const (*value).id, context)
            .map_err(|e| {
                <<__C as ::rkyv::bytecheck::rancor::Fallible>::Error as ::rkyv::bytecheck::rancor::Trace>::trace(
                    e,
                    ::rkyv::bytecheck::StructCheckContext {
                        struct_name: "ArchivedSerialHandle",
                        field_name: "id",
                    },
                )
            })?;
        <<rkyv::with::Skip as ::rkyv::with::ArchiveWith<
            Vec<u8>,
        >>::Archived as ::rkyv::bytecheck::CheckBytes<
            __C,
        >>::check_bytes(&raw const (*value).__config_buf, context)
            .map_err(|e| {
                <<__C as ::rkyv::bytecheck::rancor::Fallible>::Error as ::rkyv::bytecheck::rancor::Trace>::trace(
                    e,
                    ::rkyv::bytecheck::StructCheckContext {
                        struct_name: "ArchivedSerialHandle",
                        field_name: "__config_buf",
                    },
                )
            })?;
        ::core::result::Result::Ok(())
    }
}
#[automatically_derived]
///The resolver for an archived [`SerialHandle`]
pub struct SerialHandleResolver
where
    u64: ::rkyv::Archive,
    rkyv::with::Skip: ::rkyv::with::ArchiveWith<Vec<u8>>,
{
    id: <u64 as ::rkyv::Archive>::Resolver,
    __config_buf: <rkyv::with::Skip as ::rkyv::with::ArchiveWith<Vec<u8>>>::Resolver,
}
impl ::rkyv::Archive for SerialHandle
where
    u64: ::rkyv::Archive,
    rkyv::with::Skip: ::rkyv::with::ArchiveWith<Vec<u8>>,
{
    type Archived = ArchivedSerialHandle;
    type Resolver = SerialHandleResolver;
    #[allow(clippy::unit_arg)]
    fn resolve(&self, resolver: Self::Resolver, out: ::rkyv::Place<Self::Archived>) {
        let field_ptr = unsafe { &raw mut (*out.ptr()).id };
        let field_out = unsafe { ::rkyv::Place::from_field_unchecked(out, field_ptr) };
        <u64 as ::rkyv::Archive>::resolve(&self.id, resolver.id, field_out);
        let field_ptr = unsafe { &raw mut (*out.ptr()).__config_buf };
        let field_out = unsafe { ::rkyv::Place::from_field_unchecked(out, field_ptr) };
        <rkyv::with::Skip as ::rkyv::with::ArchiveWith<
            Vec<u8>,
        >>::resolve_with(&self.__config_buf, resolver.__config_buf, field_out);
    }
}
unsafe impl ::rkyv::traits::Portable for ArchivedSerialHandle
where
    u64: ::rkyv::Archive,
    rkyv::with::Skip: ::rkyv::with::ArchiveWith<Vec<u8>>,
    <u64 as ::rkyv::Archive>::Archived: ::rkyv::traits::Portable,
    <rkyv::with::Skip as ::rkyv::with::ArchiveWith<
        Vec<u8>,
    >>::Archived: ::rkyv::traits::Portable,
{}
#[automatically_derived]
impl<__S: ::rkyv::rancor::Fallible + ?Sized> ::rkyv::Serialize<__S> for SerialHandle
where
    u64: ::rkyv::Serialize<__S>,
    rkyv::with::Skip: ::rkyv::with::SerializeWith<Vec<u8>, __S>,
{
    fn serialize(
        &self,
        serializer: &mut __S,
    ) -> ::core::result::Result<
        <Self as ::rkyv::Archive>::Resolver,
        <__S as ::rkyv::rancor::Fallible>::Error,
    > {
        let __this = self;
        ::core::result::Result::Ok(SerialHandleResolver {
            id: <u64 as ::rkyv::Serialize<__S>>::serialize(&__this.id, serializer)?,
            __config_buf: <rkyv::with::Skip as ::rkyv::with::SerializeWith<
                Vec<u8>,
                __S,
            >>::serialize_with(&__this.__config_buf, serializer)?,
        })
    }
}
#[automatically_derived]
impl<__D: ::rkyv::rancor::Fallible + ?Sized> ::rkyv::Deserialize<SerialHandle, __D>
for ::rkyv::Archived<SerialHandle>
where
    u64: ::rkyv::Archive,
    <u64 as ::rkyv::Archive>::Archived: ::rkyv::Deserialize<u64, __D>,
    rkyv::with::Skip: ::rkyv::with::ArchiveWith<Vec<u8>>,
    rkyv::with::Skip: ::rkyv::with::DeserializeWith<
        <rkyv::with::Skip as ::rkyv::with::ArchiveWith<Vec<u8>>>::Archived,
        Vec<u8>,
        __D,
    >,
{
    fn deserialize(
        &self,
        deserializer: &mut __D,
    ) -> ::core::result::Result<SerialHandle, <__D as ::rkyv::rancor::Fallible>::Error> {
        let __this = self;
        ::core::result::Result::Ok(SerialHandle {
            id: <<u64 as ::rkyv::Archive>::Archived as ::rkyv::Deserialize<
                u64,
                __D,
            >>::deserialize(&__this.id, deserializer)?,
            __config_buf: <rkyv::with::Skip as ::rkyv::with::DeserializeWith<
                <rkyv::with::Skip as ::rkyv::with::ArchiveWith<Vec<u8>>>::Archived,
                Vec<u8>,
                __D,
            >>::deserialize_with(&__this.__config_buf, deserializer)?,
        })
    }
}
impl ::pyroduct::module_capability::CapabilityClient for SerialHandle {
    fn config_buffer(&self) -> &[u8] {
        &self.__config_buf
    }
}
impl SerialHandle {
    pub fn open(port: String, baud: u32) -> SerialHandle {
        let mut new_self = (|| {
            SerialHandle {
                id: 0,
                __config_buf: std::vec::Vec::new(),
            }
        })();
        new_self.__config_buf = ::rkyv::to_bytes::<::rkyv::rancor::Error>(&new_self)
            .expect("Failed to serialize config")
            .into_vec();
        ::pyroduct::module_capability::access::call_from_wasm::<
            SerialHandle,
            (),
            (),
            _,
        >(
            "__serial_pool__serial_server__new_client",
            Some(&new_self),
            None,
            |
                client_state_ptr: *const u8,
                client_state_len: usize,
                input_ptr: *const u8,
                input_len: usize|
            {
                unsafe {
                    wasm::__serial_pool__serial_server__new_client__wasm(
                        client_state_ptr,
                        client_state_len,
                        input_ptr,
                        input_len,
                    )
                }
            },
        );
        new_self
    }
    pub fn close(&self) -> Result<(), String> {
        ::pyroduct::module_capability::access::call_from_wasm::<
            SerialHandle,
            (),
            Result<(), String>,
            _,
        >(
            "__serial_pool__serial_server__close",
            Some(&self),
            None,
            |
                client_state_ptr: *const u8,
                client_state_len: usize,
                input_ptr: *const u8,
                input_len: usize|
            {
                unsafe {
                    wasm::__serial_pool__serial_server__close__wasm(
                        client_state_ptr,
                        client_state_len,
                        input_ptr,
                        input_len,
                    )
                }
            },
        )
    }
}
pub mod wasm {
    use super::*;
    #[link(wasm_import_module = "env")]
    unsafe extern "C" {
        pub fn __serial_pool__serial_server__new_client__wasm(
            cs_ptr: *const u8,
            cs_len: usize,
            in_ptr: *const u8,
            in_len: usize,
        ) -> *const u8;
        pub fn __serial_pool__serial_server__close__wasm(
            cs_ptr: *const u8,
            cs_len: usize,
            in_ptr: *const u8,
            in_len: usize,
        ) -> *const u8;
    }
}
pub trait SerialPool {
    fn new_client(&self, client: &SerialHandle) -> ();
    fn close(&self, client: &SerialHandle) -> Result<(), String>;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __serial_pool__serial_server__new_client__ffi(
    client_state_ptr: *const u8,
    client_state_len: usize,
    input_ptr: *const u8,
    input_len: usize,
    capability_state_ptr: *mut std::ffi::c_void,
) -> ::pyroduct::capability_host::ffi::FfiResult {
    ::pyroduct::capability::safe_call::sc_call::<
        SerialServer,
        SerialHandle,
        (),
        _,
    >(
        client_state_ptr,
        client_state_len,
        input_ptr,
        input_len,
        capability_state_ptr,
        |state, client| state.new_client(&client),
    )
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __serial_pool__serial_server__close__ffi(
    client_state_ptr: *const u8,
    client_state_len: usize,
    input_ptr: *const u8,
    input_len: usize,
    capability_state_ptr: *mut std::ffi::c_void,
) -> ::pyroduct::capability_host::ffi::FfiResult {
    ::pyroduct::capability::safe_call::sc_call::<
        SerialServer,
        SerialHandle,
        Result<(), String>,
        _,
    >(
        client_state_ptr,
        client_state_len,
        input_ptr,
        input_len,
        capability_state_ptr,
        |state, client| state.close(&client),
    )
}
const __SERIAL_POOL__SERIAL_SERVER: &'static str = "__serial_pool__serial_server";
const __SERIAL_POOL__SERIAL_SERVER__NEW_CLIENT: &'static str = "__serial_pool__serial_server__new_client";
const __SERIAL_POOL__SERIAL_SERVER__CLOSE: &'static str = "__serial_pool__serial_server__close";
const __SERIAL_POOL__SERIAL_SERVER__METHODS: [::pyroduct::capability_host::ffi::FunctionExport; 2usize] = [
    ::pyroduct::capability_host::ffi::FunctionExport {
        module: __SERIAL_POOL__SERIAL_SERVER.as_ptr(),
        module_len: __SERIAL_POOL__SERIAL_SERVER.len(),
        name: __SERIAL_POOL__SERIAL_SERVER__NEW_CLIENT.as_ptr(),
        name_len: __SERIAL_POOL__SERIAL_SERVER__NEW_CLIENT.len(),
        func: ::pyroduct::capability_host::ffi::Function::Sync(
            __serial_pool__serial_server__new_client__ffi,
        ),
    },
    ::pyroduct::capability_host::ffi::FunctionExport {
        module: __SERIAL_POOL__SERIAL_SERVER.as_ptr(),
        module_len: __SERIAL_POOL__SERIAL_SERVER.len(),
        name: __SERIAL_POOL__SERIAL_SERVER__CLOSE.as_ptr(),
        name_len: __SERIAL_POOL__SERIAL_SERVER__CLOSE.len(),
        func: ::pyroduct::capability_host::ffi::Function::Sync(
            __serial_pool__serial_server__close__ffi,
        ),
    },
];
impl state::SerialServerInit for state::SerialServer {
    fn new(config: &SerialConfig) -> Self {
        Self { next_id: 0 }
    }
    fn reset(&mut self) {
        self.next_id = 0;
    }
}
impl methods::SerialPool for state::SerialServer {
    fn new_client(&self, _client: &SerialHandle) -> () {}
    fn close(&self, _client: &SerialHandle) -> Result<(), String> {
        Ok(())
    }
}
fn main() {}
