use filecoin_proofs_api::seal::{seal_commit_phase2, SealCommitPhase1Output, SealCommitPhase2Output};
use filecoin_proofs_api::{ProverId, SectorId};
use std::sync::Arc;
use anyhow::Result;
use std::time::Duration;
use std::sync::atomic::{AtomicUsize, Ordering};
use crate::proof_rpc::task_pool::*;
use log::*;
use hex::FromHex;
use crossbeam_utils::thread;
use crossbeam_channel::tick;


pub trait Worker {
    fn seal_commit_phase2(&self,
                          phase1_output_arg: SealCommitPhase1Output,
                          prover_id: ProverId,
                          sector_id: SectorId,
    ) -> Result<SealCommitPhase2Output>;

    fn process_tasks(&self);
}

pub struct LocalWorker {
    pub worker_id: String,
    pub max_task: usize,
    pub task_pool:  Arc<dyn WorkerFetch+ Send + Sync>
}

impl LocalWorker{
    pub fn new(worker_id: String, task_pool:  Arc<dyn WorkerFetch+ Send + Sync>) -> Self {
        LocalWorker { worker_id, max_task:10, task_pool }
    }
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
        info!("worker {} start to worker and wait for new tasks", self.worker_id.clone());
        let ticker = tick(Duration::from_secs(10));
        loop {
            ticker.recv().unwrap();
            let  count = Arc::new(AtomicUsize::new(0));
            if count.load(Ordering::SeqCst) >= self.max_task {
                continue
            }
            count.fetch_add(1, Ordering::SeqCst);
            let result = self.task_pool.fetch_one_todo(self.worker_id.clone());

            match result {
                Ok(undo_task) => {
                    info!("worker {} got task {} todo", self.worker_id.clone(), undo_task.id);
                    let count_clone = Arc::clone(&count);
                    let task_pool = self.task_pool.clone();
                    thread::scope(|s| {
                        s.spawn(|_| {
                            defer! {
                                count_clone.fetch_sub(1, Ordering::SeqCst);
                            }
                            let prover_id_arg: ProverId = FromHex::from_hex(undo_task.prove_id).unwrap();
                            let sector_id_arg: SectorId = SectorId::from(undo_task.sector_id as u64);
                            let phase1_output_arg: SealCommitPhase1Output = serde_json::from_str( undo_task.phase1_output.as_str()).unwrap();
                            match self.seal_commit_phase2(phase1_output_arg, prover_id_arg, sector_id_arg){
                                Ok(proof_arg) => {
                                    let base64_proof = base64::encode(proof_arg.proof).to_string();
                                    task_pool.record_proof(self.worker_id.clone(), undo_task.id, base64_proof);
                                }
                                Err(e) => {
                                    task_pool.record_error(self.worker_id.clone(), undo_task.id, e.to_string());
                                }
                            }
                        });
                    }).unwrap();
                },
                Err(e) => {
                    error!("unable to fetch undo task {}", e)
                }
            }
        }
    }
}