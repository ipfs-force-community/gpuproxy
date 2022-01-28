use std::str::FromStr;
use filecoin_proofs_api::{ProverId, SectorId};
use crate::proof_rpc::task_pool::*;
use crate::models::{Task, Bas64Byte};
use crate::proof_rpc::resource;
use jsonrpc_core::{Result,Error, ErrorCode};
use jsonrpc_derive::rpc;

use jsonrpc_http_server::jsonrpc_core::IoHandler;
use jsonrpc_core_client::transports::http;
use std::sync::Arc;
use anyhow::anyhow;

#[rpc(client, server)]
pub trait ProofRpc {
    #[rpc(name = "Proof.SubmitTask")]
    fn submit_task(&self,
                  phase1_output: Bas64Byte,
                  miner: String,
                  prover_id: ProverId,
                  sector_id: u64,
    ) -> Result<String>;

    #[rpc(name = "Proof.GetTask")]
    fn get_task(&self, id: String) -> Result<Task>;

    #[rpc(name = "Proof.FetchTodo")]
    fn fetch_todo(&self, worker_id_arg: String) -> Result<Task> ;

    #[rpc(name = "Proof.FetchUncomplete")]
    fn fetch_uncomplte(&self, worker_id_arg: String) -> Result<Vec<Task>>;

    #[rpc(name = "Proof.GetResourceInfo")]
    fn get_resource_info(&self, resource_id_arg: String) -> Result<Vec<u8>>;

    #[rpc(name = "Proof.RecordProof")]
    fn record_proof(&self, worker_id_arg: String, tid: String, proof: String) -> Result<bool>;

    #[rpc(name = "Proof.RecordError")]
    fn record_error(&self, worker_id_arg: String, tid: String, err_msg: String) -> Result<bool>;

}

pub struct ProofImpl {
    resource: Arc<dyn resource::Resource+ Send + Sync>,
    pool: Arc<dyn Taskpool+ Send + Sync>,
}

impl ProofRpc for ProofImpl {
    fn submit_task(&self,
          phase1_output: Bas64Byte,
          miner: String,
          prover_id: ProverId,
          sector_id: u64,
    ) -> Result<String> {
        let scp1o = serde_json::from_slice(Into::<Vec<u8>>::into(phase1_output).as_slice()).unwrap();
        let addr = forest_address::Address::from_str(miner.as_str()).unwrap();
        let c2_resurce = resource::C2{
            prove_id: prover_id,
            sector_id: SectorId::from(sector_id),
            phase1_output: scp1o,
        };
        let  resource_bytes = serde_json::to_vec(&c2_resurce).unwrap();
        let resource_id = self.resource.store_resource_info(resource_bytes).unwrap();
        let tid = self.pool.addTask(addr, resource_id).unwrap();
        Ok(tid)
    }

    fn get_task(&self, id: String) -> Result<Task> {
        Ok(self.pool.fetch(id).unwrap())
    }

    fn fetch_todo(&self, worker_id_arg: String) -> Result<Task> {
        Ok(self.pool.fetch_one_todo(worker_id_arg).unwrap())
    }

    fn fetch_uncomplte(&self, worker_id_arg: String) -> Result<Vec<Task>>{
        Ok(self.pool.fetch_uncomplte(worker_id_arg).unwrap())
    }

    fn get_resource_info(&self, resource_id_arg: String) -> Result<Vec<u8>>{
        Ok(self.resource.get_resource_info(resource_id_arg).unwrap())
    }

    fn record_error(&self, worker_id_arg: String, tid: String, err_msg: String) -> Result<bool> {
      match  self.pool.record_error(worker_id_arg, tid, err_msg) {
          Some(val) => Err(
            Error{
                code: ErrorCode::InternalError,
                message: val.to_string(),
                data:None,
             }
          ),
          _ => Ok(true)
      }
    }

    fn record_proof(&self, worker_id_arg: String, tid: String, proof: String) -> Result<bool> {
        match  self.pool.record_proof(worker_id_arg, tid, proof) {
            Some(val) => Err(
              Error{
                  code: ErrorCode::InternalError,
                  message: val.to_string(),
                  data:None,
               }
            ),
            _ => Ok(true)
        }
    }
}

pub fn register(resource: Arc<dyn resource::Resource+ Send + Sync>, pool:  Arc<dyn Taskpool+ Send + Sync>) -> IoHandler {
    let mut io = IoHandler::default();
    let proof_impl = ProofImpl {resource, pool};
    io.extend_with(proof_impl.to_delegate());
    io 
}

pub fn get_worker_api(url: String) -> anyhow::Result<WrapClient> {
    match jsonrpc_core::futures_executor::block_on(http::connect::<gen_client::Client>(url.as_str())) {
        Ok(val) => Ok( WrapClient::new(val)),
        Err(e) => Err(anyhow!(e.to_string()))
    }
}

pub struct WrapClient{
    client:gen_client::Client
}

impl WrapClient {
    fn new(client:gen_client::Client) ->Self {
        return WrapClient{
            client
        }
    }
}

impl resource::Resource for WrapClient {
    fn get_resource_info(&self, resource_id_arg: String) -> anyhow::Result<Vec<u8>> {
        match jsonrpc_core::futures_executor::block_on(self.client.get_resource_info(resource_id_arg)) {
            Ok(t)=>Ok(t),
            Err(e)=>Err(anyhow!(e.to_string()))
        }
    }

    fn store_resource_info(&self, _: Vec<u8>) -> anyhow::Result<String> {
       Err(anyhow!("not support set resource in worker"))
    }
}

impl WorkerFetch for WrapClient{
    fn fetch_one_todo(&self, worker_id: String) -> anyhow::Result<Task> {
        match jsonrpc_core::futures_executor::block_on(self.client.fetch_todo(worker_id)) {
            Ok(t)=>Ok(t),
            Err(e)=>Err(anyhow!(e.to_string()))
        }
    }

    fn fetch_uncomplte(&self, worker_id_arg: String) -> anyhow:: Result<Vec<Task>> {
        match jsonrpc_core::futures_executor::block_on(self.client.fetch_uncomplte(worker_id_arg)) {
            Ok(t)=>Ok(t),
            Err(e)=>Err(anyhow!(e.to_string()))
        }
    }

     fn record_error(&self, worker_id: String, tid: String, err_msg: String) -> Option<anyhow::Error> {
         match jsonrpc_core::futures_executor::block_on(self.client.record_error(worker_id, tid, err_msg)) {
             Ok(_)=> None,
             Err(e)=>Some(anyhow!(e.to_string()))
         }
    }

     fn record_proof(&self, worker_id: String, tid: String, proof: String) -> Option<anyhow::Error> {
         match jsonrpc_core::futures_executor::block_on(self.client.record_proof(worker_id, tid, proof)) {
             Ok(_)=> None,
             Err(e)=>Some(anyhow!(e.to_string()))
         }
    }
}

pub enum ResourceOp {
    File(resource::FileResource),
    Db(TaskpoolImpl),
}