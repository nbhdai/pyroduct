use rkyv::rancor::Error as RancorError;

use crate::{BridgeError, BridgeVec, Bridgeable, DataStatus, ResultBridgeable, TypedBuf};

impl<T, E> ResultBridgeable<T, E> for Result<T, E>
where
    T: Bridgeable,
    E: Bridgeable,
{
    fn serialize(&self) -> Result<BridgeVec, RancorError> {
        match &self {
            Ok(ok_value) => ok_value.serialize(),
            Err(err_value) => {
                err_value.serialize().map(|mut e| {
                    e.set_status(DataStatus::UserError as u8);
                    e.set_error_version(e.version());
                    e.set_version(0);
                    e
                })
            }
        }
    }
    fn parse(vec: BridgeVec) -> Result<Result<TypedBuf<T>,TypedBuf<E>>, BridgeError> {
        match vec.parsed_status() {
            Ok(DataStatus::ValidData) => {
                let buf = T::unchecked_parse(vec)?;
                Ok(Ok(buf))
            }
            Ok(DataStatus::UserError) => {
                let buf = E::unchecked_parse(vec)?;
                Ok(Err(buf))
            },
            Ok(DataStatus::TransportError) => {
                let slice = vec.as_slice();
                let transport: serde_json::Value = serde_json::from_slice(slice)
                    .map_err(|e| BridgeError::RemoteError(format!("Failed to parse error JSON: {}", e)))?;
                
                Err(BridgeError::Transport(transport))
            }
            Ok(DataStatus::Utf8Error) => {
                let s = std::str::from_utf8(vec.as_slice())?;
                Err(BridgeError::RemoteError(s.to_string()))
            }
            Err(unknown) => Err(BridgeError::UnknownStatus(unknown, vec)),
        }
    }
}