use anyhow::anyhow;
use jsonrpsee::types::error::{CallError, ErrorCode};
use jsonrpsee_core::Error::Call;
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
            Some(e) => Err(jsonrpsee::core::Error::Call(CallError::Failed(anyhow!(
                "{}", e
            )))),
            _ => Ok(true),
        }
    }
}

/// this trait used in db ops, when query return Option<T>, convert to Result<T> while Some, convert to Err("not found") for None value
pub trait IfNotFound<T> {
    fn if_not_found(self, str: String) -> anyhow::Result<T>;
}

/// convert Option<T> to Result<T>
impl<T> IfNotFound<T> for Option<T> {
    fn if_not_found(self, str: String) -> anyhow::Result<T> {
        match self {
            Some(t) => Ok(t),
            _ => Err(anyhow!("not found {}", str)),
        }
    }
}

/// convert std result to jsonrpsee result
pub trait IntoJsonRpcResult<T> {
    fn invalid_params(self) -> jsonrpsee::core::RpcResult<T>;
    fn internal_call_error(self) -> jsonrpsee::core::RpcResult<T>;
}

impl<T, E> IntoJsonRpcResult<T> for Result<T, E>
where
    E: Display,
{
    fn invalid_params(self) -> jsonrpsee::core::RpcResult<T> {
        self.map_err(|e| jsonrpsee::core::Error::Call(CallError::InvalidParams(anyhow!("{}", e))))
    }

    fn internal_call_error(self) -> jsonrpsee::core::RpcResult<T> {
        self.map_err(|e| jsonrpsee::core::Error::Call(CallError::Failed(anyhow!("{}", e))))
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
        if let Err(e) = self {
            error!("{}", e.to_string())
        }
    }
}
