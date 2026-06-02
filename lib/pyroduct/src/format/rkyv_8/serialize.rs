use rkyv::rancor::{Error as RancorError, Fallible};
use rkyv::ser::allocator::{Arena, ArenaHandle};
use rkyv::ser::{Positional, Writer as RkyvWriterTrait}; // Renamed to avoid collision
use rkyv::{
    Archive,
    rancor::Strategy,
    ser::{Serializer, sharing::Share},
};
use std::cell::RefCell;
use std::marker::PhantomData;
use tracing::trace;

use crate::format::{
    PyroVec,
    format::{MutWrapper, Wrapper, Writer},
    header::MutPyroData,
};
use crate::{CapturedError, PyroError, PyroResult};

// --- Thread Local Scratch Space ---
thread_local! {
    static SCRATCH: RefCell<Arena> = RefCell::new(Arena::new());
}

// --- Rkyv Extensions for PyroVec ---
// These allow PyroVec to act as a sink for rkyv's internal serializers

impl Fallible for PyroVec {
    type Error = RancorError;
}

impl Positional for PyroVec {
    #[inline]
    fn pos(&self) -> usize {
        self.len()
    }
}

impl<E> RkyvWriterTrait<E> for PyroVec {
    #[inline]
    fn write(&mut self, bytes: &[u8]) -> Result<(), E> {
        trace!(
            bytes_len = bytes.len(),
            current_pos = self.len(),
            "RkyvWriter::write: appending bytes to PyroVec"
        );
        self.extend_from_slice(bytes);
        Ok(())
    }
}

pub struct RkyvWriter<BD, T> {
    pub(super) data: BD,
    pub(super) phantom: PhantomData<T>,
}

impl<BD: MutPyroData, T> Wrapper for RkyvWriter<BD, T> {
    type Wrapping = BD;
    fn data(&self) -> &Self::Wrapping {
        &self.data
    }
    fn into_data(self) -> Self::Wrapping {
        self.data
    }
}

impl<BD: MutPyroData, T> MutWrapper for RkyvWriter<BD, T> {
    fn data_mut(&mut self) -> &mut Self::Wrapping {
        &mut self.data
    }
}

impl<T> Writer<PyroVec, T> for RkyvWriter<PyroVec, T>
where
    T: Archive,
    for<'a> T:
        rkyv::Serialize<Strategy<Serializer<&'a mut PyroVec, ArenaHandle<'a>, Share>, RancorError>>,
{
    type HeaderValues = super::RkyvHeader;

    fn write_raw(&mut self, value: &T) -> PyroResult<usize> {
        let start_len = self.data.len();
        trace!(
            start_len,
            type_name = std::any::type_name::<T>(),
            "write_raw: starting serialization"
        );

        let bytes_written = SCRATCH.with(|scratch| {
            trace!("write_raw: acquiring thread-local scratch arena");
            let mut borrow = scratch.borrow_mut();
            let arena = &mut *borrow;

            let handle = arena.acquire();
            let share = Share::new();
            trace!("write_raw: arena handle acquired, creating serializer");

            // We serialize directly into the PyroVec (self.data)
            // The Serializer wraps the mutable reference to BD
            let mut serializer = Serializer::new(&mut self.data, handle, share);

            trace!("write_raw: invoking rkyv::api::serialize_using");
            let bytes_written =
                rkyv::api::serialize_using::<_, RancorError>(value, &mut serializer).map_err(
                    |e| {
                        trace!(error = %e, "write_raw: serialization failed");
                        PyroError::serialization(CapturedError::new(e))
                    },
                )?;

            trace!("write_raw: serialization completed successfully");
            Ok::<usize, PyroError>(bytes_written)
        })?;

        let end_len = self.data.len();
        trace!(end_len, bytes_written, "write_raw: serialization complete");
        Ok(bytes_written)
    }
}
