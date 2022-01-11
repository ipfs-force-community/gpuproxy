use filecoin_proofs_api::seal::{SealCommitPhase1Output};
use filecoin_proofs_api::{ProverId, SectorId};

use crate::models::{Task};

use jsonrpc_core::{Result, Error, ErrorCode};
use jsonrpc_derive::rpc;
use jsonrpc_http_server::jsonrpc_core::IoHandler;

use diesel::prelude::*;

use std::sync::{Mutex};

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
    ) -> Result<bool>;
}

pub struct ProofImpl {
    conn: Mutex<SqliteConnection>,
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
    ) -> Result<bool> {
        todo!()
    }
}

pub fn register(io: &mut IoHandler, conn: Mutex<SqliteConnection>) {
    let proof_impl = ProofImpl {conn};
    io.extend_with(proof_impl.to_delegate());
}
