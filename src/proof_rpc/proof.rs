use std::str::FromStr;
use filecoin_proofs_api::{ProverId};
use crate::proof_rpc::task_pool::*;
use crate::models::{Task};
use jsonrpc_core::{Result,Error, ErrorCode};
use jsonrpc_derive::rpc;
use jsonrpc_http_server::jsonrpc_core::IoHandler;
use jsonrpc_core_client::transports::http;
use std::sync::Arc;
use anyhow::anyhow;

#[rpc(client, server)]
pub trait ProofRpc {

    #[rpc(name = "PROOF.RecordProof")]
    fn record_proof(&self, tid: i64, proof: String) -> Result<bool>;
    
    #[rpc(name = "PROOF.RecordError")]
    fn record_error(&self, tid: i64, err_msg: String) -> Result<bool>;


    #[rpc(name = "PROOF.SubmitTask")]
    fn submit_task(&self,
                  phase1_output: Vec<u8>,
                  miner: String,
                  prover_id: ProverId,
                  sector_id: i64,
    ) -> Result<i64>;

    #[rpc(name = "PROOF.GetTask")]
    fn get_task(&self, id: i64) -> Result<Task>;

    #[rpc(name = "PROOF.FetchTodo")]
    fn fetch_todo(&self) -> Result<Task> ;


}

pub struct ProofImpl {
    worker_id: String,
    pool: Arc<dyn Taskpool+ Send + Sync>
}

impl ProofRpc for ProofImpl {
    fn submit_task(&self,
          phase1_output: Vec<u8>,
          miner: String,
          prover_id: ProverId,
          sector_id: i64,
    ) -> Result<i64> {
        let scp1o = serde_json::from_slice(phase1_output.as_slice()).unwrap();
        let addr = forest_address::Address::from_str(miner.as_str()).unwrap();
        let hex_prover_id = hex::encode(prover_id);
        Ok(self.pool.add(addr, self.worker_id.clone(), hex_prover_id, sector_id, scp1o).unwrap())
    }

    fn get_task(&self, id: i64) -> Result<Task> {
        Ok(self.pool.fetch(id).unwrap())
    }

    fn fetch_todo(&self) -> Result<Task> {
        Ok(self.pool.fetch_one_todo().unwrap())
    }

    
    fn record_error(&self, tid: i64, err_msg: String) -> Result<bool> {
      match  self.pool.record_error(tid, err_msg) {
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

    fn record_proof(&self, tid: i64, proof: String) -> Result<bool> {
        match  self.pool.record_proof(tid, proof) {
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

pub fn register(worker_id: String, pool:  Arc<dyn Taskpool+ Send + Sync>) -> IoHandler {
    let mut io = IoHandler::default();
    let proof_impl = ProofImpl {worker_id, pool};
    io.extend_with(proof_impl.to_delegate());
    io 
}

pub async fn get_worker_api(url: String) -> WrapClient {
    WrapClient::new(http::connect::<gen_client::Client>(url.as_str()).await.unwrap())
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

impl WorkerFetch for WrapClient{
    fn fetch_one_todo(&self) -> anyhow::Result<Task> {
        match jsonrpc_core::futures_executor::block_on(self.client.fetch_todo()) {
            Ok(t)=>Ok(t),
            Err(e)=>Err(anyhow!(e.to_string()))
        }
    }

     fn record_error(&self, tid: i64, err_msg: String) -> Option<anyhow::Error> {
         match jsonrpc_core::futures_executor::block_on(self.client.record_error(tid, err_msg)) {
             Ok(_)=> None,
             Err(e)=>Some(anyhow!(e.to_string()))
         }
    }

     fn record_proof(&self, tid: i64, proof: String) -> Option<anyhow::Error> {
         match jsonrpc_core::futures_executor::block_on(self.client.record_proof(tid, proof)) {
             Ok(_)=> None,
             Err(e)=>Some(anyhow!(e.to_string()))
         }
    }
}