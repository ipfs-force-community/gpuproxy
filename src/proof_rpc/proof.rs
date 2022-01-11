use filecoin_proofs_api::seal::{SealCommitPhase1Output};
use filecoin_proofs_api::{ProverId, SectorId};
use crate::task_pool::Taskpool;
use crate::models::{Task};

use jsonrpc_core::{Result, Error, ErrorCode};
use jsonrpc_derive::rpc;
use jsonrpc_http_server::jsonrpc_core::IoHandler;

use diesel::prelude::*;

use std::sync::{Mutex};
use serde_json::json;

#[rpc]
pub trait ProofRpc {
    /// Adds two numbers and returns a result
    #[rpc(name = "PROOF.Add")]
    fn add(&self, a: u64, b: u64) -> Result<u64>;

    fn submit_task(&self,
                  phase1_output: SealCommitPhase1Output,
                  miner: String,
                  prover_id: ProverId,
                  sector_id: SectorId,
    ) -> Result<i64>;
}

pub struct ProofImpl {
    pool: Taskpool,
}

impl ProofRpc for ProofImpl {
    fn add(&self, a: u64, b: u64) -> Result<u64> {
        println!("receive request {} + {}", a, b);
        Ok(a + b)
    }

    fn submit_task(&self,
          phase1_output: SealCommitPhase1Output,
          miner: String,
          prover_id: ProverId,
          sector_id: SectorId,
    ) -> Result<i64> {
        let hex_prover_id = hex::encode(prover_id);
        let phase1_json = json!(phase1_output);
        let row_id =  self.pool.add(miner, hex_prover_id, sector_id.0 as i64, phase1_json.to_string()).unwrap();
        Ok(row_id)
    }
}

pub fn register(io: &mut IoHandler, pool: Taskpool) {
    let proof_impl = ProofImpl {pool};
    io.extend_with(proof_impl.to_delegate());
}
