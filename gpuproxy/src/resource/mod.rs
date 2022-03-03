use crate::config;
use crate::utils::Base64Byte;
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
use uuid::Uuid;

/// The data required for computing c2 type tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C2Resource {
    pub prover_id: ProverId,
    pub sector_id: SectorId,
    pub c1out: SealCommitPhase1Output,
}

/// Persist task related data and can implement specific storage media to save data
#[async_trait]
pub trait Resource {
    async fn has_resource(&self, resource_id: String) -> Result<bool>;
    async fn get_resource_info(&self, resource_id: String) -> Result<Base64Byte>;
    async fn store_resource_info(&self, resource_id: String, resource: Vec<u8>) -> Result<String>;
}

/// Use files to persist task data
pub struct FileResource {
    root: String,
}

impl FileResource {
    pub fn new(root: String) -> Self {
        return FileResource { root };
    }
}

unsafe impl Send for FileResource {}
unsafe impl Sync for FileResource {}

#[async_trait]
impl Resource for FileResource {
    /// Check if the resource exit
    async fn has_resource(&self, resource_id: String) -> Result<bool> {
        let new_path = Path::new(self.root.as_str()).join(resource_id.clone());
        Ok(new_path.is_file())
    }

    /// get task resource by resource id
    async fn get_resource_info(&self, resource_id: String) -> Result<Base64Byte> {
        let new_path = Path::new(self.root.as_str()).join(resource_id.clone());
        let filename = new_path.to_str().ok_or(anyhow!(
            "unable to find file for resource {} in directory {}",
            resource_id.clone(),
            self.root.clone()
        ))?;
        let mut f = File::open(filename)?;
        let metadata = fs::metadata(filename)?;
        let mut buffer = vec![0; metadata.len() as usize];
        f.read(&mut buffer)?;
        Ok(Base64Byte::new(buffer))
    }

    /// save task resource to file system
    async fn store_resource_info(&self, resource_id: String, resource: Vec<u8>) -> Result<String> {
        let new_path = Path::new(self.root.as_str()).join(resource_id.clone());
        fs::write(new_path, resource)?;
        Ok(resource_id)
    }
}

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
