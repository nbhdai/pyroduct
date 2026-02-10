use crate::format::PyroHeaderValues;

mod pyro;
mod buffers;
mod deserialize;
mod parse;
mod serialize;
pub use rkyv;
mod common;

pub use pyro::Rkyv;
pub use deserialize::RkyvReceiver;
pub use parse::RkyvParser;
pub use serialize::RkyvWriter;

pub use buffers::{TypedPyroView, TypedBuf};
pub struct RkyvHeader;

impl PyroHeaderValues for RkyvHeader {
    const OK_CODE: crate::header::DataStatus = crate::header::DataStatus::RkyvValid;
    const ERR_CODE: crate::header::DataStatus = crate::header::DataStatus::RkyvError;
}
