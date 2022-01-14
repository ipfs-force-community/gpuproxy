use filecoin_proofs_api::seal::{SealCommitPhase1Output};
use filecoin_proofs_api::{ProverId};
use crate::task_pool::{Taskpool};

use jsonrpc_core::{Result};
use jsonrpc_derive::rpc;
use jsonrpc_http_server::jsonrpc_core::IoHandler;

use std::sync::Arc;
use serde_json::json;

#[rpc]
pub trait ProofRpc {
    #[rpc(name = "PROOF.SubmitTask")]
    fn submit_task(&self,
                  phase1_output: SealCommitPhase1Output,
                  miner: String,
                  prover_id: ProverId,
                  sector_id: i64,
    ) -> Result<i64>;
}

pub struct ProofImpl {
    pool: Arc<dyn Taskpool+ Send + Sync>
}

impl ProofRpc for ProofImpl {
    fn submit_task(&self,
          phase1_output: SealCommitPhase1Output,
          miner: String,
          prover_id: ProverId,
          sector_id: i64,
    ) -> Result<i64> {
        let hex_prover_id = hex::encode(prover_id);
        let phase1_json = json!(phase1_output);
        let row_id =  self.pool.add(miner, hex_prover_id, sector_id, phase1_json.to_string()).unwrap();
        Ok(row_id)
    }
}

pub fn register(io: &mut IoHandler, pool:  Arc<dyn Taskpool+ Send + Sync>) {
    let proof_impl = ProofImpl {pool};
    io.extend_with(proof_impl.to_delegate());
}