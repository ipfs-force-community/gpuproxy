use filecoin_proofs_api::seal::{seal_commit_phase2, SealCommitPhase1Output, SealCommitPhase2Output};
use filecoin_proofs_api::{ProverId, SectorId};
use std::sync::{Mutex};
use crate::models::{Task};
use anyhow::Result;
use diesel::prelude::*;
use crate::models::schema::tasks::dsl::*;
use std::time::Duration;

use crate::task_pool;
use ticker::Ticker;
use log::*;
use hex::{FromHex};
use jsonrpc_core::serde::Serialize;

pub trait Worker {
    fn seal_commit_phase2(&self,
                          phase1_output_arg: SealCommitPhase1Output,
                          prover_id: ProverId,
                          sector_id: SectorId,
    ) -> Result<SealCommitPhase2Output>;

    fn process_tasks(&self);
}

pub struct LocalWorker {
    task_pool: dyn task_pool::Taskpool
}

impl Worker for LocalWorker {
    fn seal_commit_phase2(&self,
                          phase1_output_arg: SealCommitPhase1Output,
                          prover_id_arg: ProverId,
                          sector_id_arg: SectorId,
    ) -> Result<SealCommitPhase2Output> {
          seal_commit_phase2(phase1_output_arg, prover_id_arg, sector_id_arg)
    }

    fn process_tasks(&self) {
        let ticker = Ticker::new((0..), Duration::from_secs(1));
        for _ in ticker {
            let result = self.task_pool.fetch_one_todo();
            match result {
                Ok(undo_task) => {
                    //todo in another thread

                    let prover_id_arg: ProverId = FromHex::from_hex(undo_task.prove_id).unwrap();
                    let sector_id_arg: SectorId = SectorId::from(undo_task.sector_id as u64);
                    let phase1_output_arg: SealCommitPhase1Output = serde_json::from_str( undo_task.phase1_output.as_str()).unwrap();
                    let proof_arg = self.seal_commit_phase2(phase1_output_arg, prover_id_arg, sector_id_arg).unwrap();
                    let bytes = serde_json::to_string(&proof_arg).unwrap();
                    self.task_pool.record_proof(undo_task.id, bytes);
                },
                Err(e) => {
                    error!("unable to fetch undo task {}", e)
                }
            }
        }
    }
}