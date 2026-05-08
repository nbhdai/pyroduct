use crate::format::{format::PyroHeaderValues, header::DataStatus};

mod bridge;
mod buffers;
mod deserialize;
mod parse;
mod serialize;
pub use rkyv;
mod common;

pub use bridge::Rkyv;
pub use deserialize::RkyvReceiver;
pub use parse::RkyvParser;
pub use serialize::RkyvWriter;

pub use buffers::{TypedBuf, TypedPyroRef};
pub struct RkyvHeader;

impl PyroHeaderValues for RkyvHeader {
    const OK_CODE: DataStatus = DataStatus::RkyvValid;
    const ERR_CODE: DataStatus = DataStatus::RkyvError;
}
