use std::fmt::Display;
use anyhow::anyhow;

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
    fn reverse_map_err(self) -> jsonrpc_core::Result<bool>;
}

impl<E> ReveseOption for Option<E>
    where
        E: Display
{
    fn reverse_map_err(self) -> jsonrpc_core::Result<bool> {
        match self {
            Some(val) => Err(
                jsonrpc_core::Error{
                    code: jsonrpc_core::ErrorCode::InternalError,
                    message: val.to_string(),
                    data:None,
                }
            ),
            _ => Ok(true)
        }
    }
}

pub trait IntoJsonRpcResult<T> {
    fn to_jsonrpc_result(self, code: jsonrpc_core::ErrorCode) -> jsonrpc_core::Result<T>;
}

impl<T, E> IntoJsonRpcResult<T> for Result<T, E>
    where
        E: Display
{
    fn to_jsonrpc_result(self, code: jsonrpc_core::ErrorCode) -> jsonrpc_core::Result<T> {
        self.map_err(|e|jsonrpc_core::Error{
            code: code,
            message: e.to_string(),
            data:None,
        })
    }
}

