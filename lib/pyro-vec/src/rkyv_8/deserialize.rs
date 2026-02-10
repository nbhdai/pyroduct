use std::{marker::PhantomData, ops::Deref};

use rkyv::Deserialize;

use crate::{PyroError, PyroResult, CapturedError, format::Receiver};

pub struct RkyvReceiver<T> {
    pub(super) tpe: PhantomData<T>,
}

impl<T> RkyvReceiver<T> {
    pub fn new() -> Self {
        Self { tpe: PhantomData }
    }
}

// --- Implementation for Owned Buffers (TypedBuf) ---

impl<T> Receiver<<T as rkyv::Archive>::Archived, T> for RkyvReceiver<T>
where
    T: rkyv::Archive,
    // Ensure the archived type matches the static lifetime requirement of TypedBuf
    T::Archived: 'static,
    // Match the specific deserialization bounds defined in common.rs
    T::Archived: Deserialize<T, rkyv::rancor::Strategy<rkyv::de::Pool, rkyv::rancor::Error>>,
{
    fn receive<D>(&mut self, data: &D) -> PyroResult<T>
    where
        D: Deref<Target = <T as rkyv::Archive>::Archived>,
    {
        rkyv::deserialize::<T, rkyv::rancor::Error>(data.deref())
            .map_err(|e| PyroError::deserialization(CapturedError::new(e)))
    }
}
