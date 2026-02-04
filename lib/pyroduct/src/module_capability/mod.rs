use std::ops::Deref;

use rkyv::util::AlignedVec;

pub mod access;
pub mod error;

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

impl<T> Client<T> {
    pub fn new(data: T, buffer: AlignedVec) -> Self {
        Self {
            data,
            __config_buf: buffer,
        }
    }
    pub fn buffer(&self) -> &AlignedVec {
        &self.__config_buf
    }
}
