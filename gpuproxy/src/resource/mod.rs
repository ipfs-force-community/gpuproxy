use crate::config;
use crate::proxy_rpc::db_ops::ResourceRepo;
use crate::utils::Base64Byte;
use anyhow::Context;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use filecoin_proofs_api::seal::SealCommitPhase1Output;
use filecoin_proofs_api::ProverId;
use filecoin_proofs_api::SectorId;
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;

/// The data required for computing c2 type tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C2Resource {
    pub prover_id: ProverId,
    pub sector_id: SectorId,
    pub c1out: SealCommitPhase1Output,
}

//ResourceOp and ResourceRepo have the same methods in current implementation
pub trait ResourceOp: ResourceRepo {}
impl<T> ResourceOp for T where T: ResourceRepo {}

/// Use files to persist task data
pub struct FileResource {
    root: String,
}

impl FileResource {
    pub fn new(root: String) -> Self {
        FileResource { root }
    }
}

unsafe impl Send for FileResource {}
unsafe impl Sync for FileResource {}

#[async_trait]
impl ResourceRepo for FileResource {
    async fn has_resource(&self, resource_id: String) -> Result<bool> {
        let new_path = Path::new(self.root.as_str()).join(resource_id);
        Ok(new_path.is_file())
    }

    async fn get_resource_info(&self, resource_id: String) -> Result<Base64Byte> {
        let new_path = Path::new(self.root.as_str()).join(resource_id);
        let content =
            fs::read(&new_path).with_context(|| format!("read file: {}", new_path.display()))?;
        Ok(Base64Byte::new(content))
    }

    async fn store_resource_info(&self, resource_id: String, resource: Vec<u8>) -> Result<String> {
        let new_path = Path::new(self.root.as_str()).join(resource_id.clone());
        fs::write(new_path, resource)?;
        Ok(resource_id)
    }
}

/// Use database to persist task data
pub struct DbResource {
    resource_repo: Arc<dyn ResourceRepo + Send + Sync>,
}

impl DbResource {
    pub fn new(resource_repo: Arc<dyn ResourceRepo + Send + Sync>) -> Self {
        DbResource { resource_repo }
    }
}

unsafe impl Send for DbResource {}
unsafe impl Sync for DbResource {}

#[async_trait]
impl ResourceRepo for DbResource {
    /// Check if the resource exit
    async fn has_resource(&self, resource_id: String) -> Result<bool> {
        return self.resource_repo.has_resource(resource_id).await;
    }

    /// get task resource by resource id
    async fn get_resource_info(&self, resource_id: String) -> Result<Base64Byte> {
        return self.resource_repo.get_resource_info(resource_id).await;
    }

    /// save task resource to file system
    async fn store_resource_info(&self, resource_id: String, resource: Vec<u8>) -> Result<String> {
        return self
            .resource_repo
            .store_resource_info(resource_id, resource)
            .await;
    }
}

/// Use rpc to get task data (not support set task data by rpc)
pub type RpcResource = DbResource;

#[test]
pub fn test_de() {
    let test_str = include_str!("./c2proxy_base64");

    use super::*;

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct C2Input {
        pub c1out: SealCommitPhase1Output,
        pub prover_id: ProverId,
        pub sector_id: SectorId,
        pub miner_id: u64,
    }

    let c2_input_json = base64::decode(test_str).unwrap();
    serde_json::from_slice::<C2Resource>(&c2_input_json).unwrap();
}
