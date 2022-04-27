use crate::proxy_rpc::db_ops::*;
use filecoin_proofs_api::{ProverId, SectorId};
use std::str::FromStr;

use crate::resource;
use crate::utils::Base64Byte;
use crate::utils::{IntoAnyhow, IntoJsonRpcResult, ReveseOption};
use anyhow::anyhow;
use bytes::BufMut;
use entity::tasks as Tasks;
use entity::worker_info as WorkerInfos;
use entity::TaskType;
use entity::{resource_info as ResourceInfos, TaskState};
use jsonrpsee::core::{async_trait, client::Subscription, RpcResult};
use jsonrpsee::http_client::{HttpClient, HttpClientBuilder};
use jsonrpsee::proc_macros::rpc;
use jsonrpsee::types::error::ErrorCode::{InternalError, InvalidParams};
use jsonrpsee::RpcModule;
use std::sync::Arc;
use uuid::Uuid;
use ResourceInfos::Model as ResourceInfo;
use Tasks::Model as Task;
use WorkerInfos::Model as WorkerInfo;

pub const ONE_GIB: u32 = 1024 * 1024 * 1024;

#[rpc(server, client)]
pub trait ProxyRpc {
    #[method(name = "Proof.SubmitC2Task")]
    async fn submit_c2_task(
        &self,
        phase1_output: Base64Byte,
        miner: String,
        prover_id: ProverId,
        sector_id: u64,
    ) -> RpcResult<String>;

    #[method(name = "Proof.AddTask")]
    async fn add_task(
        &self,
        miner: String,
        task_type: entity::TaskType,
        param: Base64Byte,
    ) -> RpcResult<String>;

    #[method(name = "Proof.AddTaskWithExitResource")]
    async fn add_task_with_exit_resource(
        &self,
        miner: String,
        task_type: entity::TaskType,
        resouce_id: String,
    ) -> RpcResult<String>;

    #[method(name = "Proof.GetTask")]
    async fn get_task(&self, id: String) -> RpcResult<Task>;

    #[method(name = "Proof.FetchTodo")]
    async fn fetch_todo(
        &self,
        worker_id_arg: String,
        types: Option<Vec<entity::TaskType>>,
    ) -> RpcResult<Task>;

    #[method(name = "Proof.FetchUncomplete")]
    async fn fetch_uncompleted(&self, worker_id_arg: String) -> RpcResult<Vec<Task>>;

    #[method(name = "Proof.GetResourceInfo")]
    async fn get_resource_info(&self, resource_id_arg: String) -> RpcResult<Base64Byte>;

    #[method(name = "Proof.RecordProof")]
    async fn record_proof(
        &self,
        worker_id_arg: String,
        tid: String,
        proof: Base64Byte,
    ) -> RpcResult<bool>;

    #[method(name = "Proof.RecordError")]
    async fn record_error(
        &self,
        worker_id_arg: String,
        tid: String,
        err_msg: String,
    ) -> RpcResult<bool>;

    #[method(name = "Proof.ListTask")]
    async fn list_task(
        &self,
        worker_id_arg: Option<String>,
        state: Option<Vec<entity::TaskState>>,
    ) -> RpcResult<Vec<Task>>;

    #[method(name = "Proof.UpdateStatusById")]
    async fn update_status_by_id(
        &self,
        tids: Vec<String>,
        status: entity::TaskState,
    ) -> RpcResult<bool>;
}

pub struct ProxyImpl {
    resource: Arc<dyn resource::Resource + Send + Sync>,
    pool: Arc<dyn DbOp + Send + Sync>,
}

impl ProxyImpl {
    async fn add_task_inner(
        &self,
        addr: forest_address::Address,
        task_type: TaskType,
        resource_bytes: Vec<u8>,
    ) -> RpcResult<String> {
        let resource_id = Uuid::new_v5(&Uuid::NAMESPACE_OID, &resource_bytes).to_string();
        if !self.resource.has_resource(resource_id.clone()).await? {
            let _ = self
                .resource
                .store_resource_info(resource_id.clone(), resource_bytes)
                .await
                .internal_call_error()?;
        }

       let task_id =  ProxyImpl::generate_task_id(addr, task_type, resource_id.clone());
        if !self.pool.has_task(task_id.clone()).await? {
            self.pool
                .clone()
                .add_task(task_id.clone(), addr, task_type, resource_id)
                .await
                .internal_call_error()?;
        }
        Ok(task_id)
    }

    fn generate_task_id(addr: forest_address::Address, task_type: TaskType, resource_id: String)-> String{
        let mut buf = bytes::BytesMut::new();
        buf.put_slice(&addr.payload_bytes());
        buf.put_i32(task_type.into());
        buf.put_slice(resource_id.clone().as_bytes());
        return Uuid::new_v5(&Uuid::NAMESPACE_OID, buf.as_ref()).to_string();
    }
}

#[async_trait]
impl ProxyRpcServer for ProxyImpl {
    /// Submit C2 task this api if used for golang, compatable with Goland invoke parameters and return
    async fn submit_c2_task(
        &self,
        phase1_output: Base64Byte,
        miner: String,
        prover_id: ProverId,
        sector_id: u64,
    ) -> RpcResult<String> {
        let scp1o = serde_json::from_slice(Into::<Vec<u8>>::into(phase1_output).as_slice())
            .invalid_params()?;
        let addr = forest_address::Address::from_str(miner.as_str()).invalid_params()?;
        let c2_resource = resource::C2Resource {
            prover_id,
            sector_id: SectorId::from(sector_id),
            c1out: scp1o,
        };

        let resource_bytes = serde_json::to_vec(&c2_resource).invalid_params()?;

        self.add_task_inner(addr, TaskType::C2, resource_bytes)
            .await
    }

    /// Add specify task into gpuproxy, params must be base64 encoded bytes
    async fn add_task(
        &self,
        miner: String,
        task_type: TaskType,
        param: Base64Byte,
    ) -> RpcResult<String> {
        let addr = forest_address::Address::from_str(miner.as_str()).invalid_params()?;
        //check
        match task_type {
            TaskType::C2 => {
                serde_json::from_slice::<resource::C2Resource>(&param.0).invalid_params()?;
            }
        }

        self.add_task_inner(addr, task_type, param.0).await
    }

    /// add task with exit resource, if use fs storage, you can storage c2resource in fs, and upload task with the resource name
    async fn add_task_with_exit_resource(
        &self,
        miner: String,
        task_type: entity::TaskType,
        resource_id: String,
    ) -> RpcResult<String>{
        let addr = forest_address::Address::from_str(miner.as_str()).invalid_params()?;
        let has_resource = self.resource.has_resource(resource_id.clone()).await.internal_call_error()?;
        if !has_resource {
            return Err(format!("resouce {} not exit", resource_id.clone())).invalid_params()?
        }

        let task_id = ProxyImpl::generate_task_id(addr, task_type, resource_id.clone());
        if !self.pool.has_task(task_id.clone()).await? {
            self.pool
                .clone()
                .add_task(task_id.clone(), addr, task_type, resource_id)
                .await
                .internal_call_error()?;
        }
        Ok(task_id)
    }

    /// Get task by id
    async fn get_task(&self, id: String) -> RpcResult<Task> {
        self.pool.fetch(id).await.internal_call_error()
    }

    /// Fetch a undo task and mark it to running
    async fn fetch_todo(
        &self,
        worker_id_arg: String,
        types: Option<Vec<entity::TaskType>>,
    ) -> RpcResult<Task> {
        self.pool
            .fetch_one_todo(worker_id_arg, types)
            .await
            .internal_call_error()
    }

    /// Fetch uncompleted task for specify worker
    async fn fetch_uncompleted(&self, worker_id_arg: String) -> RpcResult<Vec<Task>> {
        self.pool
            .fetch_uncompleted(worker_id_arg)
            .await
            .internal_call_error()
    }

    /// Get resource data by resource id
    async fn get_resource_info(&self, resource_id_arg: String) -> RpcResult<Base64Byte> {
        self.resource
            .get_resource_info(resource_id_arg)
            .await
            .internal_call_error()
    }

    /// Record task result after completed computing task
    async fn record_proof(
        &self,
        worker_id_arg: String,
        tid: String,
        proof: Base64Byte,
    ) -> RpcResult<bool> {
        self.pool
            .record_proof(worker_id_arg, tid, proof.0)
            .await
            .reverse_map_err()
    }

    /// Record task error while completing
    async fn record_error(
        &self,
        worker_id_arg: String,
        tid: String,
        err_msg: String,
    ) -> RpcResult<bool> {
        self.pool
            .record_error(worker_id_arg, tid, err_msg)
            .await
            .reverse_map_err()
    }

    /// List task by worker id and task state
    async fn list_task(
        &self,
        worker_id_arg: Option<String>,
        state: Option<Vec<entity::TaskState>>,
    ) -> RpcResult<Vec<Task>> {
        self.pool
            .list_task(worker_id_arg, state)
            .await
            .internal_call_error()
    }

    /// Update task status by task ids
    async fn update_status_by_id(
        &self,
        tids: Vec<String>,
        state: entity::TaskState,
    ) -> RpcResult<bool> {
        self.pool
            .update_status_by_id(tids, state)
            .await
            .reverse_map_err()
    }
}

/// new proxy apu impl and get rpc moudle
pub fn register(
    resource: Arc<dyn resource::Resource + Send + Sync>,
    pool: Arc<dyn DbOp + Send + Sync>,
) -> RpcModule<ProxyImpl> {
    let proof_impl = ProxyImpl { resource, pool };
    proof_impl.into_rpc()
}

/// get proxy api by url
pub async fn get_proxy_api(url: String) -> anyhow::Result<WrapClient> {
    HttpClientBuilder::default()
        .max_request_body_size(ONE_GIB)
        .build(url.as_str())
        .map(|val| WrapClient { client: val })
        .anyhow()
}

/// WrapClient for rpc error, convert RpcResult to anyhow Result
pub struct WrapClient {
    client: HttpClient,
}

#[async_trait]
impl resource::Resource for WrapClient {
    async fn has_resource(&self, resource_id: String) -> anyhow::Result<bool> {
        Err(anyhow!("not support set resource in worker"))
    }

    async fn get_resource_info(&self, resource_id_arg: String) -> anyhow::Result<Base64Byte> {
        self.client
            .get_resource_info(resource_id_arg)
            .await
            .anyhow()
    }

    async fn store_resource_info(&self, _: String, _: Vec<u8>) -> anyhow::Result<String> {
        Err(anyhow!("not support set resource in worker"))
    }
}

#[async_trait]
impl WorkerFetch for WrapClient {
    async fn fetch_one_todo(
        &self,
        worker_id: String,
        types: Option<Vec<entity::TaskType>>,
    ) -> anyhow::Result<Task> {
        self.client.fetch_todo(worker_id, types).await.anyhow()
    }

    async fn fetch_uncompleted(&self, worker_id_arg: String) -> anyhow::Result<Vec<Task>> {
        self.client.fetch_uncompleted(worker_id_arg).await.anyhow()
    }

    async fn record_error(
        &self,
        worker_id: String,
        tid: String,
        err_msg: String,
    ) -> Option<anyhow::Error> {
        self.client
            .record_error(worker_id, tid, err_msg)
            .await
            .err()
            .map(|e| anyhow!(e.to_string()))
    }

    async fn record_proof(
        &self,
        worker_id: String,
        tid: String,
        proof: Vec<u8>,
    ) -> Option<anyhow::Error> {
        self.client
            .record_proof(worker_id, tid, Base64Byte(proof))
            .await
            .err()
            .map(|e| anyhow!(e.to_string()))
    }
}

#[async_trait]
pub trait GpuServiceRpcClient {
    async fn submit_c2_task(
        &self,
        phase1_output: Base64Byte,
        miner: String,
        prover_id: ProverId,
        sector_id: u64,
    ) -> anyhow::Result<String>;

    async fn add_task(
        &self,
        miner: String,
        task_type: TaskType,
        param: Base64Byte,
    ) -> anyhow::Result<String>;

    async fn get_task(&self, id: String) -> anyhow::Result<Task>;

    async fn fetch_todo(
        &self,
        worker_id_arg: String,
        types: Option<Vec<entity::TaskType>>,
    ) -> anyhow::Result<Task>;

    async fn fetch_uncompleted(&self, worker_id_arg: String) -> anyhow::Result<Vec<Task>>;

    async fn get_resource_info(&self, resource_id_arg: String) -> anyhow::Result<Base64Byte>;

    async fn record_proof(
        &self,
        worker_id_arg: String,
        tid: String,
        proof: Base64Byte,
    ) -> anyhow::Result<bool>;

    async fn record_error(
        &self,
        worker_id_arg: String,
        tid: String,
        err_msg: String,
    ) -> anyhow::Result<bool>;

    async fn list_task(
        &self,
        worker_id_arg: Option<String>,
        state: Option<Vec<entity::TaskState>>,
    ) -> anyhow::Result<Vec<Task>>;

    async fn update_status_by_id(
        &self,
        tids: Vec<String>,
        state: entity::TaskState,
    ) -> anyhow::Result<bool>;
}

#[async_trait]
impl GpuServiceRpcClient for WrapClient {
    async fn submit_c2_task(
        &self,
        phase1_output: Base64Byte,
        miner: String,
        prover_id: ProverId,
        sector_id: u64,
    ) -> anyhow::Result<String> {
        self.client
            .submit_c2_task(phase1_output, miner, prover_id, sector_id)
            .await
            .anyhow()
    }

    async fn add_task(
        &self,
        miner: String,
        task_type: TaskType,
        param: Base64Byte,
    ) -> anyhow::Result<String> {
        self.client.add_task(miner, task_type, param).await.anyhow()
    }

    async fn get_task(&self, id: String) -> anyhow::Result<Task> {
        self.client.get_task(id).await.anyhow()
    }

    async fn fetch_todo(
        &self,
        worker_id_arg: String,
        types: Option<Vec<entity::TaskType>>,
    ) -> anyhow::Result<Task> {
        self.client.fetch_todo(worker_id_arg, types).await.anyhow()
    }

    async fn fetch_uncompleted(&self, worker_id_arg: String) -> anyhow::Result<Vec<Task>> {
        self.client.fetch_uncompleted(worker_id_arg).await.anyhow()
    }

    async fn get_resource_info(&self, resource_id_arg: String) -> anyhow::Result<Base64Byte> {
        self.client
            .get_resource_info(resource_id_arg)
            .await
            .anyhow()
    }

    async fn record_proof(
        &self,
        worker_id_arg: String,
        tid: String,
        proof: Base64Byte,
    ) -> anyhow::Result<bool> {
        self.client
            .record_proof(worker_id_arg, tid, proof)
            .await
            .anyhow()
    }

    async fn record_error(
        &self,
        worker_id_arg: String,
        tid: String,
        err_msg: String,
    ) -> anyhow::Result<bool> {
        self.client
            .record_error(worker_id_arg, tid, err_msg)
            .await
            .anyhow()
    }

    async fn list_task(
        &self,
        worker_id_arg: Option<String>,
        state: Option<Vec<entity::TaskState>>,
    ) -> anyhow::Result<Vec<Task>> {
        self.client.list_task(worker_id_arg, state).await.anyhow()
    }

    async fn update_status_by_id(
        &self,
        tids: Vec<String>,
        state: entity::TaskState,
    ) -> anyhow::Result<bool> {
        self.client.update_status_by_id(tids, state).await.anyhow()
    }
}
