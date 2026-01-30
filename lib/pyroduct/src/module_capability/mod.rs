use std::ops::Deref;

use rkyv::util::AlignedVec;

pub mod access;
pub mod error;
pub mod panic;

pub trait CapabilityClient {
    fn config_buffer(&self) -> &[u8];
}

pub struct Client<T> {
    data: T,
    __config_buf: AlignedVec,
}

impl<T> Deref for Client<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}