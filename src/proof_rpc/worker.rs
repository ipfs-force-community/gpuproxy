use filecoin_proofs_api::seal::{seal_commit_phase2};
use std::sync::Arc;
use anyhow::Result;
use std::time::Duration;
use std::sync::atomic::{AtomicUsize, Ordering};
use crate::proof_rpc::task_pool::*;
use crate::models::*;
use crate::proof_rpc::resource::{*};
use log::*;
use crossbeam_channel::tick;
use std::thread as stdthread;

pub trait Worker {
    fn fetch_one_todo(&self) ->Result<Task>;

    fn process_tasks(self) -> stdthread::JoinHandle<()>;
}

pub struct LocalWorker {
    pub worker_id: String,
    pub max_task: usize,
    pub task_pool:  Arc<dyn WorkerFetch+ Send + Sync>,
    pub resource:  Arc<dyn Resource+ Send + Sync>,
}

unsafe impl Send for LocalWorker {}

impl LocalWorker{
    pub fn new(max_task: usize, worker_id: String, resource: Arc<dyn Resource+ Send + Sync>, task_pool:  Arc<dyn WorkerFetch+ Send + Sync>) -> Self {
        LocalWorker { worker_id, max_task, task_pool, resource}
    }
}

impl Worker for LocalWorker {
    fn fetch_one_todo(&self) -> Result<Task> {
        let uncomplete_task_result = self.task_pool.fetch_uncomplte(self.worker_id.clone());
        if let Ok(uncomplete_tasks) = uncomplete_task_result {
            if uncomplete_tasks.len() > 0 {
                let fetch_work = uncomplete_tasks[0].clone();
                info!("worker {} fetch uncomplete task {}", self.worker_id, fetch_work.id);
                return Ok(fetch_work);
            }
        }
        self.task_pool.fetch_one_todo(self.worker_id.clone())
    }
    fn process_tasks(self) -> stdthread::JoinHandle<()>{
       let handler =  stdthread::spawn(move ||{
                info!("worker {} start to worker and wait for new tasks", self.worker_id);
                let ticker = tick(Duration::from_secs(10));
                let count = Arc::new(AtomicUsize::new(0));
                loop {
                    ticker.recv().unwrap();
                    let cur_size = count.load(Ordering::SeqCst);
                    if  cur_size  >= self.max_task {
                        info!("has reach the max number of c2 tasks {} {}", cur_size, self.max_task);
                        continue
                    }
                    match self.fetch_one_todo() {
                        Ok(undo_task) => {
                            count.fetch_add(1, Ordering::SeqCst);
                            let count_clone = count.clone();
                            let task_pool = self.task_pool.clone();
                            let worker_id = self.worker_id.clone();
                            let resource_result = self.resource.get_resource_info(undo_task.resource_id.clone());
                            if let Err(e) = resource_result {
                                error!("unable to get resource of {}, reason:{}", undo_task.resource_id, e.to_string());
                                continue
                            }
                            let resource =  resource_result.unwrap();
                            stdthread::spawn(move|| {
                                defer! {
                                    count_clone.fetch_sub(1, Ordering::SeqCst);
                                }

                                if undo_task.task_type == TaskType::C2 {
                                    let c2: C2 = serde_json::from_slice( &resource).unwrap();
                                    info!("worker {} start to do task {}, size {}", worker_id.clone(), undo_task.id, u64::from(c2.phase1_output.registered_proof.sector_size()));
                                    match seal_commit_phase2(c2.phase1_output, c2.prove_id, c2.sector_id,){
                                        Ok(proof_arg) => {
                                            info!("worker {} complted {} success", worker_id.clone(), undo_task.id);
                                            let base64_proof = base64::encode(proof_arg.proof).to_string();
                                            task_pool.record_proof(worker_id.clone(), undo_task.id, base64_proof);
                                        }
                                        Err(e) => {
                                            info!("worker {} execute {} fail reason {}", worker_id.clone(), undo_task.id, e.to_string());
                                            task_pool.record_error(worker_id.clone(), undo_task.id, e.to_string());
                                        }
                                    }
                                }

                            });
                            
                        },
                        Err(e) => {
                            error!("unable to fetch undo task {}", e)
                        }
                    }
                }
            }); 
        info!("worker has started");
        handler
    }
}