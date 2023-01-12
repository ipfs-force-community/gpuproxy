use anyhow::{anyhow, Result};
use bytes::BufMut;
use bytes::BytesMut;
use entity::TaskType;
use jsonrpsee::types::error::{CallError, ErrorCode};
use jsonrpsee_core::Error::Call;
use log::error;
use std::fmt::Display;
use std::fs::File;
use std::path::Path;
use std::pin::Pin;
use uuid::Uuid;

mod base64bytes;
pub use base64bytes::Base64Byte;

/// convert any std result to anyhow result
pub trait IntoAnyhow<T> {
    fn anyhow(self) -> Result<T>;
}

impl<T, E> IntoAnyhow<T> for Result<T, E>
where
    E: Display,
{
    fn anyhow(self) -> Result<T> {
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
    fn if_not_found(self, str: String) -> Result<T>;
}

/// convert Option<T> to Result<T>
impl<T> IfNotFound<T> for Option<T> {
    fn if_not_found(self, str: String) -> Result<T> {
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

pub fn gen_resource_id(resource_bytes: &[u8]) -> String {
    Uuid::new_v5(&Uuid::NAMESPACE_OID, resource_bytes).to_string()
}

pub fn gen_task_id(
    addr: forest_address::Address,
    task_type: TaskType,
    resource_bytes: &[u8],
) -> String {
    let resource_id = gen_resource_id(resource_bytes);
    let mut buf = BytesMut::new();
    buf.put_slice(&addr.payload_bytes());
    buf.put_i32(task_type.into());
    buf.put_slice(resource_id.as_bytes());
    Uuid::new_v5(&Uuid::NAMESPACE_OID, buf.as_ref()).to_string()
}

pub async fn ensure_db_file(url: &str) -> Result<()> {
    if !url.starts_with("sqlite") {
        return Ok(());
    }

    let sub_url = url
        .trim_start_matches("sqlite://")
        .trim_start_matches("sqlite:");

    let mut database_and_params = sub_url.splitn(2, '?');
    let database = database_and_params.next().unwrap_or_default();
    if database != ":memory:" {
        let path = Path::new(database);
        if !path.exists() {
            File::create(path)?;
        }
    }
    Ok(())
}
