use crate::config;
use crate::utils::base64bytes::Base64Byte;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C2Resource {
    pub prover_id: ProverId,
    pub sector_id: SectorId,
    pub c1out: SealCommitPhase1Output,
}

#[async_trait]
pub trait Resource {
    async fn get_resource_info(&self, resource_id: String) -> Result<Base64Byte>;
    async fn store_resource_info(&self, resource: Vec<u8>) -> Result<String>;
}

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

    async fn store_resource_info(&self, resource: Vec<u8>) -> Result<String> {
        let resource_id = Uuid::new_v4().to_string();
        let new_path = Path::new(self.root.as_str()).join(resource_id.clone());
        fs::write(new_path, resource)?;
        Ok(resource_id)
    }
}
