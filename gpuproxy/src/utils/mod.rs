use std::fmt::Display;
use std::pin::Pin;
use anyhow::anyhow;
pub mod base64bytes;

pub trait IntoAnyhow<T> {
    fn anyhow(self) -> anyhow::Result<T>;
}


impl<T, E> IntoAnyhow<T> for Result<T, E>
    where
        E: Display
{
    fn anyhow(self) -> anyhow::Result<T> {
        self.map_err(|e| anyhow!(e.to_string()))
    }
}


pub trait ReveseOption {
    fn reverse_map_err(self) -> jsonrpsee::core::RpcResult<bool>;
}

impl<E> ReveseOption for Option<E>
    where
        E: Display
{
    fn reverse_map_err(self) -> jsonrpsee::core::RpcResult<bool> {
        match self {
            Some(val) => Err(
                jsonrpsee::core::Error::Custom(val.to_string())
               /* jsonrpsee::core::{
                    code: jsonrpc_core::ErrorCode::InternalError,
                    message: val.to_string(),
                    data:None,
                }*/
            ),
            _ => Ok(true)
        }
    }
}

pub trait IfNotFound<T> {
    fn if_not_found(self) -> anyhow::Result<T>;
}

impl<T> IfNotFound<T> for Option<T>
{
    fn if_not_found(self) -> anyhow::Result<T> {
        match self {
            Some(t) => Ok(t),
            _ => Err(anyhow!("not found"))
        }
    }
}

pub trait IntoJsonRpcResult<T> {
    fn to_jsonrpc_result(self, code: jsonrpc_core::ErrorCode) -> jsonrpsee::core::RpcResult<T>;
}

impl<T, E> IntoJsonRpcResult<T> for Result<T, E>
    where
        E: Display
{
    fn to_jsonrpc_result(self, code: jsonrpc_core::ErrorCode) ->  jsonrpsee::core::RpcResult<T> {
        self.map_err(|e|jsonrpsee::core::Error::Custom(e.to_string()))
    }
}
