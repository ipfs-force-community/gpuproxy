use std::fmt::Display;
use anyhow::anyhow;

pub trait IntoAnyhow<T> {
    fn anyhow(self) -> anyhow::Result<T>;
}


/*impl<T> IntoAnyhow<T> for jsonrpc_core_client::RpcResult<T> {
    fn anyhow(self) -> anyhow::Result<T> {
        self.map_err(|e| anyhow!(e.to_string()))
    }
}*/

impl<T, E> IntoAnyhow<T> for Result<T, E>
    where
        E: Display
{
    fn anyhow(self) -> anyhow::Result<T> {
        self.map_err(|e| anyhow!(e.to_string()))
    }
}