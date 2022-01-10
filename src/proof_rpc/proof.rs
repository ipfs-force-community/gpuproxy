use filecoin_proofs_api::seal::{seal_commit_phase2, SealCommitPhase1Output, SealCommitPhase2Output};
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

    #[rpc(name = "PROOF.SealCommitPhase2")]
    fn seal_commit_phase2(&self,
    phase1_output: SealCommitPhase1Output,
    prover_id: ProverId,
    sector_id: SectorId,
    ) -> Result<SealCommitPhase2Output>;

    fn submitTask(&self,
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
/*
    fn seal_commit_phase1(&self,
        cache_path_str: String,
        replica_path_str: String,
        prover_id: ProverId,
        sector_id: SectorId,
        ticket: Ticket,
        seed: Ticket,
        pre_commit: SealPreCommitPhase2Output,
        piece_infos: Vec<PieceInfo>,
    ) -> Result<SealCommitPhase1Output> {
        let cache_path = Path::new(cache_path_str.as_str());
        let replica_path = Path::new(replica_path_str.as_str());

        match  seal_commit_phase1(cache_path, replica_path, prover_id, sector_id,ticket,seed, pre_commit, &piece_infos){
            Ok(v) => Ok(v),
            Err(e) => {
                let mut err = Error::new(ErrorCode::ServerError(503));
                err.message = e.to_string();
                Err(err)
            }
        }
    }*/

    fn seal_commit_phase2(&self,
    phase1_output: SealCommitPhase1Output,
    prover_id: ProverId,
    sector_id: SectorId,
    ) -> Result<SealCommitPhase2Output> {
        match  seal_commit_phase2(phase1_output, prover_id, sector_id){
            Ok(v) => Ok(v),
            Err(e) => {
                let mut err = Error::new(ErrorCode::ServerError(503));
                err.message = e.to_string();
                Err(err)
            }
        }
    }

    fn submitTask(&self,
          phase1_output: SealCommitPhase1Output,
          miner: String,
          prover_id: ProverId,
          sector_id: SectorId,
    ) -> Result<bool> {
        Ok(true)
    }
}

pub fn register(io: &mut IoHandler, conn: Mutex<SqliteConnection>) {
    let proof_impl = ProofImpl {conn};
    io.extend_with(proof_impl.to_delegate());
}
