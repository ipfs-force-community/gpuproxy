use std::str::FromStr;
use filecoin_proofs_api::{ProverId};
use crate::proof_rpc::task_pool::{Taskpool};
use crate::models::{Task};
use jsonrpc_core::{Result};
use jsonrpc_derive::rpc;
use jsonrpc_http_server::jsonrpc_core::IoHandler;
use jsonrpc_core_client::transports::local;

use std::sync::Arc;

#[rpc(client, server)]
pub trait ProofRpc {
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
    fn fetch_todo(&self) -> Result<Task>;
}

pub struct ProofImpl {
    worker_id: Option<String>,
    pool: Option<Arc<dyn Taskpool+ Send + Sync>>
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
        Ok(self.pool.as_ref().unwrap().add(addr, self.worker_id.as_ref().unwrap().clone(), hex_prover_id, sector_id, scp1o).unwrap())
    }

    fn get_task(&self, id: i64) -> Result<Task> {
        Ok(self.pool.as_ref().unwrap().fetch(id).unwrap())
    }

    fn fetch_todo(&self) -> Result<Task> {
        Ok(self.pool.as_ref().unwrap().fetch_one_todo().unwrap())
    }
}

pub fn register(io: &mut IoHandler, worker_id: String, pool:  Arc<dyn Taskpool+ Send + Sync>) {
    let proof_impl = ProofImpl {worker_id: Some(worker_id), pool:Some(pool)};
    io.extend_with(proof_impl.to_delegate());
}

pub fn get_client(url: String) {
    let mut io = IoHandler::new();
    let proof_impl = ProofImpl{worker_id:None, pool: None};
    io.extend_with(proof_impl.to_delegate());

    let (client, _) = local::connect::<gen_client::Client, _, _>(io);
}