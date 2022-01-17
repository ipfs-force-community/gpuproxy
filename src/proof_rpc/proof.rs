use std::str::FromStr;
use filecoin_proofs_api::{ProverId};
use crate::task_pool::{Taskpool};
use crate::models::{Task};

use jsonrpc_core::{Result};
use jsonrpc_derive::rpc;
use jsonrpc_http_server::jsonrpc_core::IoHandler;

use std::sync::Arc;

#[rpc]
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
}

pub struct ProofImpl {
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
        Ok(self.pool.add(addr, hex_prover_id, sector_id, scp1o).unwrap())
    }

    fn get_task(&self, id: i64) -> Result<Task> {
        Ok(self.pool.fetch(id).unwrap())
    }
}

pub fn register(io: &mut IoHandler, pool:  Arc<dyn Taskpool+ Send + Sync>) {
    let proof_impl = ProofImpl {pool};
    io.extend_with(proof_impl.to_delegate());
}