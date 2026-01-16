pub mod access;
pub mod error;
pub mod panic;

pub trait CapabilityClient {
    fn config_buffer(&self) -> &[u8];
}
