use crate::proxy_rpc::db_ops::*;
use filecoin_proofs_api::{ProverId, SectorId};
use log::info;
use std::env;
use std::str::FromStr;

use crate::utils::Base64Byte;
use crate::utils::{IntoAnyhow, IntoJsonRpcResult, ReveseOption};
use crate::{resource, utils};
use anyhow::{anyhow, Result};
use entity::TaskType;
use humantime::parse_duration;
use std::time::Duration;

use entity::tasks as Tasks;
use Tasks::Model as Task;

use entity::worker_info as WorkerInfos;
use WorkerInfos::Model as WorkerInfo;

use entity::workers_state::Model as WorkerState;
use entity::{resource_info as ResourceInfos, TaskState};

use hyper::Uri;
use jsonrpsee::core::{async_trait, client::Subscription, RpcResult};
use jsonrpsee::http_client::{HttpClient, HttpClientBuilder};
use jsonrpsee::proc_macros::rpc;
use jsonrpsee::types::error::ErrorCode::{InternalError, InvalidParams};
use jsonrpsee::RpcModule;
use resource::ResourceOp;
use std::sync::Arc;
use uuid::Uuid;
use ResourceInfos::Model as ResourceInfo;

pub const ONE_GIB: u32 = 1024 * 1024 * 1024;

#[rpc(server, client)]
pub trait ProxyRpc {
    #[method(name = "Proof.SubmitC2Task")]
    async fn submit_c2_task(
        &self,
        phase1_output: Base64Byte,
        miner: String,
        comment: String,
        prover_id: ProverId,
        sector_id: u64,
    ) -> RpcResult<String>;

    #[method(name = "Proof.AddTask")]
    async fn add_task(
        &self,
        miner: String,
        comment: String,
        task_type: entity::TaskType,
        param: Base64Byte,
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

    #[method(name = "Proof.RecordProof")]
    async fn record_proof(
        &self,
        worker_id_arg: String,
        tid: String,
        proof: Base64Byte,
    ) -> RpcResult<()>;

    #[method(name = "Proof.RecordError")]
    async fn record_error(
        &self,
        worker_id_arg: String,
        tid: String,
        err_msg: String,
    ) -> RpcResult<()>;

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
    ) -> RpcResult<()>;

    #[method(name = "Proof.ReportWorkerInfo")]
    async fn report_worker_info(
        &self,
        worker_id_arg: String,
        ips: String,
        support_types: String,
    ) -> RpcResult<()>;

    #[method(name = "Proof.GetResourceInfo")]
    async fn get_resource_info(&self, resource_id_arg: String) -> RpcResult<Base64Byte>;

    #[method(name = "Proof.ListWorker")]
    async fn list_worker(&self) -> RpcResult<Vec<WorkerState>>;

    #[method(name = "Proof.DeleteWorkerByWorkerId")]
    async fn delete_worker_by_worker_id(&self, worker_id_arg: String) -> RpcResult<()>;

    #[method(name = "Proof.DeleteWorkerById")]
    async fn delete_worker_by_id(&self, id: String) -> RpcResult<()>;

    #[method(name = "Proof.GetWorkerByWorkerId")]
    async fn get_worker_by_worker_id(&self, worker_id_arg: String) -> RpcResult<WorkerState>;

    #[method(name = "Proof.GetWorkerById")]
    async fn get_worker_by_id(&self, id: String) -> RpcResult<WorkerState>;

    #[method(name = "Proof.GetOfflineWorker")]
    async fn get_offline_worker(&self, dur: i64) -> RpcResult<Vec<WorkerState>>;
}

pub struct ProxyImpl {
    resource: Arc<dyn resource::ResourceOp + Send + Sync>,
    pool: Arc<dyn Repo + Send + Sync>,
}

impl ProxyImpl {
    async fn add_task_inner(
        &self,
        addr: forest_address::Address,
        comment: String,
        task_type: TaskType,
        resource_bytes: Vec<u8>,
    ) -> RpcResult<String> {
        let resource_id = utils::gen_resource_id(&resource_bytes);
        if !self.resource.has_resource(resource_id.clone()).await? {
            let _ = self
                .resource
                .store_resource_info(resource_id.clone(), resource_bytes.clone())
                .await
                .internal_call_error()?;
        }

        let task_id = utils::gen_task_id(addr, task_type, &resource_bytes);
        if !self.pool.has_task(task_id.clone()).await? {
            self.pool
                .clone()
                .add_task(task_id.clone(), addr, task_type, resource_id, comment)
                .await
                .internal_call_error()?;
        }
        Ok(task_id)
    }
}

#[async_trait]
impl ProxyRpcServer for ProxyImpl {
    /// Submit C2 task this api if used for golang, compatable with Goland invoke parameters and return
    async fn submit_c2_task(
        &self,
        phase1_output: Base64Byte,
        miner: String,
        comment: String,
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

        self.add_task_inner(addr, comment, TaskType::C2, resource_bytes)
            .await
    }

    /// Add specify task into gpuproxy, params must be base64 encoded bytes
    async fn add_task(
        &self,
        miner: String,
        comment: String,
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

        self.add_task_inner(addr, comment, task_type, param.0).await
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
    ) -> RpcResult<()> {
        let task = self.pool.fetch(tid.clone()).await.internal_call_error()?;
        self.pool
            .record_proof(worker_id_arg, tid, proof.0)
            .await
            .internal_call_error()?;
        self.resource
            .delete_resource(task.resource_id)
            .await
            .internal_call_error()
    }

    /// Record task error while completing
    async fn record_error(
        &self,
        worker_id_arg: String,
        tid: String,
        err_msg: String,
    ) -> RpcResult<()> {
        self.pool
            .record_error(worker_id_arg, tid, err_msg)
            .await
            .internal_call_error()
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
    ) -> RpcResult<()> {
        self.pool
            .update_status_by_id(tids, state)
            .await
            .internal_call_error()
    }

    async fn report_worker_info(
        &self,
        worker_id_arg: String,
        ips: String,
        support_types: String,
    ) -> RpcResult<()> {
        self.pool
            .report_worker_info(worker_id_arg, ips, support_types)
            .await
            .internal_call_error()
    }

    async fn list_worker(&self) -> RpcResult<Vec<WorkerState>> {
        self.pool.list_worker().await.internal_call_error()
    }

    async fn delete_worker_by_worker_id(&self, worker_id_arg: String) -> RpcResult<()> {
        self.pool
            .delete_worker_by_worker_id(worker_id_arg)
            .await
            .internal_call_error()
    }

    async fn delete_worker_by_id(&self, id: String) -> RpcResult<()> {
        self.pool
            .delete_worker_by_id(id)
            .await
            .internal_call_error()
    }

    async fn get_worker_by_worker_id(&self, worker_id_arg: String) -> RpcResult<WorkerState> {
        self.pool
            .get_worker_by_worker_id(worker_id_arg)
            .await
            .internal_call_error()
    }

    async fn get_worker_by_id(&self, id: String) -> RpcResult<WorkerState> {
        self.pool.get_worker_by_id(id).await.internal_call_error()
    }

    async fn get_offline_worker(&self, dur: i64) -> RpcResult<Vec<WorkerState>> {
        self.pool
            .get_offline_worker(dur)
            .await
            .internal_call_error()
    }
}

/// new proxy apu impl and get rpc moudle
pub fn register(
    resource: Arc<dyn resource::ResourceOp + Send + Sync>,
    pool: Arc<dyn Repo + Send + Sync>,
) -> RpcModule<ProxyImpl> {
    let proof_impl = ProxyImpl { resource, pool };
    proof_impl.into_rpc()
}

/// get proxy api by url
pub async fn get_proxy_api(url: String) -> Result<WrapClient> {
    let uri = Uri::from_str(&url)?;
    let new_url = if uri.scheme().is_none() {
        "http://".to_owned() + &url
    } else {
        url
    };

    let duration_str = env::var("HTTP_TIMEOUT").unwrap_or("60s".to_owned());
    let duration = parse_duration(duration_str.as_str())?;
    info!("http timeout is {:?}", duration);

    HttpClientBuilder::default()
        .request_timeout(duration)
        .max_request_body_size(ONE_GIB)
        .build(new_url.as_str())
        .map(|val| WrapClient { client: val })
        .anyhow()
}

/// WrapClient for rpc error, convert RpcResult to anyhow Result
pub struct WrapClient {
    client: HttpClient,
}

#[async_trait]
impl ResourceRepo for WrapClient {
    async fn has_resource(&self, resource_id: String) -> Result<bool> {
        Err(anyhow!("not support set resource in rpc"))
    }

    async fn get_resource_info(&self, resource_id: String) -> Result<Base64Byte> {
        self.client.get_resource_info(resource_id).await.anyhow()
    }

    async fn store_resource_info(&self, resource_id: String, resource: Vec<u8>) -> Result<String> {
        Err(anyhow!("not support store resource in rpc"))
    }

    async fn delete_resource(&self, resource_id: String) -> Result<()> {
        Err(anyhow!("not support store resource in rpc"))
    }
}

//just for better code completion
#[async_trait]
pub trait GpuServiceRpcClient {
    async fn submit_c2_task(
        &self,
        phase1_output: Base64Byte,
        miner: String,
        comment: String,
        prover_id: ProverId,
        sector_id: u64,
    ) -> Result<String>;

    async fn add_task(
        &self,
        miner: String,
        comment: String,
        task_type: TaskType,
        param: Base64Byte,
    ) -> Result<String>;

    async fn get_task(&self, id: String) -> Result<Task>;

    async fn fetch_todo(
        &self,
        worker_id_arg: String,
        types: Option<Vec<entity::TaskType>>,
    ) -> Result<Task>;

    async fn fetch_one_todo(
        &self,
        worker_id: String,
        types: Option<Vec<entity::TaskType>>,
    ) -> Result<Task>;

    async fn fetch_uncompleted(&self, worker_id_arg: String) -> Result<Vec<Task>>;

    async fn get_resource_info(&self, resource_id_arg: String) -> Result<Vec<u8>>;

    async fn record_proof(&self, worker_id_arg: String, tid: String, proof: Vec<u8>) -> Result<()>;

    async fn record_error(&self, worker_id_arg: String, tid: String, err_msg: String)
        -> Result<()>;

    async fn list_task(
        &self,
        worker_id_arg: Option<String>,
        state: Option<Vec<entity::TaskState>>,
    ) -> Result<Vec<Task>>;

    async fn update_status_by_id(&self, tids: Vec<String>, state: entity::TaskState) -> Result<()>;

    async fn report_worker_info(
        &self,
        worker_id_arg: String,
        ips: String,
        support_types: String,
    ) -> Result<()>;

    async fn list_worker(&self) -> Result<Vec<WorkerState>>;

    async fn delete_worker_by_worker_id(&self, worker_id_arg: String) -> Result<()>;

    async fn delete_worker_by_id(&self, id: String) -> Result<()>;

    async fn get_worker_by_worker_id(&self, worker_id_arg: String) -> Result<WorkerState>;

    async fn get_worker_by_id(&self, id: String) -> Result<WorkerState>;

    async fn get_offline_worker(&self, dur: i64) -> Result<Vec<WorkerState>>;
}

#[async_trait]
impl GpuServiceRpcClient for WrapClient {
    async fn submit_c2_task(
        &self,
        phase1_output: Base64Byte,
        miner: String,
        comment: String,
        prover_id: ProverId,
        sector_id: u64,
    ) -> Result<String> {
        self.client
            .submit_c2_task(phase1_output, miner, comment, prover_id, sector_id)
            .await
            .anyhow()
    }

    async fn add_task(
        &self,
        miner: String,
        comment: String,
        task_type: TaskType,
        param: Base64Byte,
    ) -> Result<String> {
        self.client
            .add_task(miner, comment, task_type, param)
            .await
            .anyhow()
    }

    async fn get_task(&self, id: String) -> Result<Task> {
        self.client.get_task(id).await.anyhow()
    }

    async fn fetch_todo(
        &self,
        worker_id_arg: String,
        types: Option<Vec<entity::TaskType>>,
    ) -> Result<Task> {
        self.client.fetch_todo(worker_id_arg, types).await.anyhow()
    }

    async fn fetch_one_todo(
        &self,
        worker_id: String,
        types: Option<Vec<entity::TaskType>>,
    ) -> Result<Task> {
        self.client.fetch_todo(worker_id, types).await.anyhow()
    }

    async fn fetch_uncompleted(&self, worker_id_arg: String) -> Result<Vec<Task>> {
        self.client.fetch_uncompleted(worker_id_arg).await.anyhow()
    }

    async fn get_resource_info(&self, resource_id_arg: String) -> Result<Vec<u8>> {
        self.client
            .get_resource_info(resource_id_arg)
            .await
            .map(|v| v.0)
            .anyhow()
    }

    async fn record_proof(&self, worker_id_arg: String, tid: String, proof: Vec<u8>) -> Result<()> {
        self.client
            .record_proof(worker_id_arg, tid, Base64Byte::new(proof))
            .await
            .anyhow()
    }

    async fn record_error(
        &self,
        worker_id_arg: String,
        tid: String,
        err_msg: String,
    ) -> Result<()> {
        self.client
            .record_error(worker_id_arg, tid, err_msg)
            .await
            .anyhow()
    }

    async fn list_task(
        &self,
        worker_id_arg: Option<String>,
        state: Option<Vec<entity::TaskState>>,
    ) -> Result<Vec<Task>> {
        self.client.list_task(worker_id_arg, state).await.anyhow()
    }

    async fn update_status_by_id(&self, tids: Vec<String>, state: entity::TaskState) -> Result<()> {
        self.client.update_status_by_id(tids, state).await.anyhow()
    }

    async fn report_worker_info(
        &self,
        worker_id_arg: String,
        ips: String,
        support_types: String,
    ) -> Result<()> {
        self.client
            .report_worker_info(worker_id_arg, ips, support_types)
            .await
            .anyhow()
    }

    async fn list_worker(&self) -> Result<Vec<WorkerState>> {
        self.client.list_worker().await.anyhow()
    }

    async fn delete_worker_by_worker_id(&self, worker_id_arg: String) -> Result<()> {
        self.client
            .delete_worker_by_worker_id(worker_id_arg)
            .await
            .anyhow()
    }

    async fn delete_worker_by_id(&self, id: String) -> Result<()> {
        self.client.delete_worker_by_id(id).await.anyhow()
    }

    async fn get_worker_by_worker_id(&self, worker_id_arg: String) -> Result<WorkerState> {
        self.client
            .get_worker_by_worker_id(worker_id_arg)
            .await
            .anyhow()
    }

    async fn get_worker_by_id(&self, id: String) -> Result<WorkerState> {
        self.client.get_worker_by_id(id).await.anyhow()
    }

    async fn get_offline_worker(&self, dur: i64) -> Result<Vec<WorkerState>> {
        self.client.get_offline_worker(dur).await.anyhow()
    }
}
