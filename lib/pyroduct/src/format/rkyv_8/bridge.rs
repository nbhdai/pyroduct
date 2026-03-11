use std::marker::PhantomData;

use rkyv::Archive;
use rkyv::bytecheck::CheckBytes;
use rkyv::de::Pool;
use rkyv::rancor::{Error as RancorError, Strategy};
use rkyv::ser::Serializer;
use rkyv::ser::allocator::ArenaHandle;
use rkyv::ser::sharing::Share;
use rkyv::validation::Validator;
use rkyv::validation::archive::ArchiveValidator;
use rkyv::validation::shared::SharedValidator;

use crate::PyroVec;
use crate::format::{PyroFormat, PyroZeroCopyFormat, UserHeaderValues};
use crate::header::PROTOCOL_VERSION;
use crate::view::PyroView;

/// Acts as the factory and configuration source for Rkyv-based PyroVecs.
pub struct Rkyv<T> {
    tpe: PhantomData<T>,
}

// ─── PyroFormat (base: owned parsing + serialization) ──────────────────────

impl<T> PyroFormat<T> for Rkyv<T>
where
    T: Archive + UserHeaderValues,
    T::Archived: 'static,
    T::Archived:
        for<'a> CheckBytes<Strategy<Validator<ArchiveValidator<'a>, SharedValidator>, RancorError>>,
    for<'a> T:
        rkyv::Serialize<Strategy<Serializer<&'a mut PyroVec, ArenaHandle<'a>, Share>, RancorError>>,
    <T as Archive>::Archived: rkyv::Deserialize<T, Strategy<Pool, rkyv::rancor::Error>>,
{
    const WIRE_FORMAT: u8 = PROTOCOL_VERSION; // 1

    type HeaderValues = super::RkyvHeader;
    type ParsedType = <T as rkyv::Archive>::Archived;

    type VecParser = super::RkyvParser<PyroVec, T>;
    type VecWriter = super::RkyvWriter<PyroVec, T>;
    type ViewParser<'a> = super::RkyvParser<PyroView<'a>, T>;

    fn new() -> Self {
        Self { tpe: PhantomData }
    }

    fn new_writer(data: PyroVec) -> Self::VecWriter {
        super::RkyvWriter {
            data,
            phantom: PhantomData,
        }
    }

    fn vec_parser(data: PyroVec) -> Self::VecParser {
        super::RkyvParser {
            data,
            phantom: PhantomData,
        }
    }

    fn view_parser<'a>(data: PyroView<'a>) -> Self::ViewParser<'a> {
        super::RkyvParser {
            data,
            phantom: PhantomData,
        }
    }
}

// ─── PyroZeroCopyFormat (extension: borrowed view parsing) ─────────────────

impl<T> PyroZeroCopyFormat<T> for Rkyv<T>
where
    T: Archive + UserHeaderValues,
    T::Archived: 'static,
    T::Archived:
        for<'a> CheckBytes<Strategy<Validator<ArchiveValidator<'a>, SharedValidator>, RancorError>>,
    for<'a> T:
        rkyv::Serialize<Strategy<Serializer<&'a mut PyroVec, ArenaHandle<'a>, Share>, RancorError>>,
    <T as Archive>::Archived: rkyv::Deserialize<T, Strategy<Pool, rkyv::rancor::Error>>,
{
    type Receiver = super::RkyvReceiver<T>;
    fn receiver() -> Self::Receiver {
        super::RkyvReceiver::new()
    }
}
