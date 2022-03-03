use anyhow::anyhow;
use jsonrpsee::types::error::ErrorCode;
use log::error;
use std::fmt::Display;
use std::pin::Pin;

mod base64bytes;
pub use base64bytes::Base64Byte;

/// convert any std result to anyhow result
pub trait IntoAnyhow<T> {
    fn anyhow(self) -> anyhow::Result<T>;
}

impl<T, E> IntoAnyhow<T> for Result<T, E>
where
    E: Display,
{
    fn anyhow(self) -> anyhow::Result<T> {
        self.map_err(|e| anyhow!(e.to_string()))
    }
}

/// Convert Option<Error> => Result<bool>
pub trait ReveseOption {
    fn reverse_map_err(self) -> jsonrpsee::core::RpcResult<bool>;
}

impl<E> ReveseOption for Option<E>
where
    E: Display,
{
    fn reverse_map_err(self) -> jsonrpsee::core::RpcResult<bool> {
        match self {
            Some(val) => Err(
                jsonrpsee::core::Error::Custom(val.to_string()), /* jsonrpsee::core::{
                                                                     code: jsonrpc_core::ErrorCode::InternalError,
                                                                     message: val.to_string(),
                                                                     data:None,
                                                                 }*/
            ),
            _ => Ok(true),
        }
    }
}

/// this trait used in db ops, when query return Option<T>, convert to Result<T> while Some, convert to Err("not found") for None value
pub trait IfNotFound<T> {
    fn if_not_found(self) -> anyhow::Result<T>;
}

/// convert Option<T> to Result<T>
impl<T> IfNotFound<T> for Option<T> {
    fn if_not_found(self) -> anyhow::Result<T> {
        match self {
            Some(t) => Ok(t),
            _ => Err(anyhow!("not found")),
        }
    }
}

/// convert std result to jsonrpsee result
pub trait IntoJsonRpcResult<T> {
    fn to_jsonrpc_result(self, code: ErrorCode) -> jsonrpsee::core::RpcResult<T>;
}

impl<T, E> IntoJsonRpcResult<T> for Result<T, E>
where
    E: Display,
{
    fn to_jsonrpc_result(self, code: ErrorCode) -> jsonrpsee::core::RpcResult<T> {
        self.map_err(|e| jsonrpsee::core::Error::Custom(e.to_string()))
    }
}

/// use to log error message and ignore the val while Ok
pub trait LogErr {
    fn log_error(self);
}

impl<T, E> LogErr for Result<T, E>
where
    E: Display,
{
    fn log_error(self) {
        match self {
            Err(e) => error!("{}", e.to_string()),
            _ => {}
        }
    }
}
