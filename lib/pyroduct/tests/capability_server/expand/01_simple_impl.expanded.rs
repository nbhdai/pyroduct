#[rkyv(crate = ::pyroduct::rkyv)]
pub struct GreeterClient {}
#[automatically_derived]
///An archived [`GreeterClient`]
#[bytecheck(crate = ::pyroduct::rkyv::bytecheck)]
#[repr(C)]
pub struct ArchivedGreeterClient {}
#[automatically_derived]
unsafe impl<
    __C: ::pyroduct::rkyv::bytecheck::rancor::Fallible + ?::core::marker::Sized,
> ::pyroduct::rkyv::bytecheck::CheckBytes<__C> for ArchivedGreeterClient
where
    <__C as ::pyroduct::rkyv::bytecheck::rancor::Fallible>::Error: ::pyroduct::rkyv::bytecheck::rancor::Trace,
{
    unsafe fn check_bytes(
        value: *const Self,
        context: &mut __C,
    ) -> ::core::result::Result<
        (),
        <__C as ::pyroduct::rkyv::bytecheck::rancor::Fallible>::Error,
    > {
        ::core::result::Result::Ok(())
    }
}
#[automatically_derived]
///The resolver for an archived [`GreeterClient`]
pub struct GreeterClientResolver {}
impl ::pyroduct::rkyv::Archive for GreeterClient {
    type Archived = ArchivedGreeterClient;
    type Resolver = GreeterClientResolver;
    const COPY_OPTIMIZATION: ::pyroduct::rkyv::traits::CopyOptimization<Self> = unsafe {
        ::pyroduct::rkyv::traits::CopyOptimization::enable_if(
            0 == ::core::mem::size_of::<GreeterClient>(),
        )
    };
    #[allow(clippy::unit_arg)]
    fn resolve(
        &self,
        resolver: Self::Resolver,
        out: ::pyroduct::rkyv::Place<Self::Archived>,
    ) {}
}
unsafe impl ::pyroduct::rkyv::traits::Portable for ArchivedGreeterClient {}
#[automatically_derived]
impl<__S: ::pyroduct::rkyv::rancor::Fallible + ?Sized> ::pyroduct::rkyv::Serialize<__S>
for GreeterClient {
    fn serialize(
        &self,
        serializer: &mut __S,
    ) -> ::core::result::Result<
        <Self as ::pyroduct::rkyv::Archive>::Resolver,
        <__S as ::pyroduct::rkyv::rancor::Fallible>::Error,
    > {
        let __this = self;
        ::core::result::Result::Ok(GreeterClientResolver {})
    }
}
#[automatically_derived]
impl<
    __D: ::pyroduct::rkyv::rancor::Fallible + ?Sized,
> ::pyroduct::rkyv::Deserialize<GreeterClient, __D>
for ::pyroduct::rkyv::Archived<GreeterClient> {
    fn deserialize(
        &self,
        deserializer: &mut __D,
    ) -> ::core::result::Result<
        GreeterClient,
        <__D as ::pyroduct::rkyv::rancor::Fallible>::Error,
    > {
        let __this = self;
        ::core::result::Result::Ok(GreeterClient {})
    }
}
fn main() {}
