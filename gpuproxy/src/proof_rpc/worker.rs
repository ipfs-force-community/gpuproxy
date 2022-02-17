use filecoin_proofs_api::seal::{seal_commit_phase2};
use std::sync::Arc;
use anyhow::Result;
use std::time::Duration;
use std::sync::atomic::{AtomicUsize, Ordering};
use crate::proof_rpc::db_ops::*;
use log::*;
use crossbeam_channel::tick;
use std::thread as stdthread;
use crate::resource::{C2Resource, Resource};

use entity::tasks as Tasks;
use entity::resource_info as ResourceInfos;
use entity::tasks::TaskType;
use entity::worker_info as WorkerInfos;
use Tasks::Model as Task;
use ResourceInfos::Model as ResourceInfo;
use WorkerInfos::Model as WorkerInfo;
use async_trait::async_trait;

#[async_trait]
pub trait Worker {
    async fn fetch_one_todo(&self) -> Result<Task>;

     fn process_tasks(self);
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

#[async_trait]
impl Worker for LocalWorker {
    async fn fetch_one_todo(&self) -> Result<Task> {
        let un_complete_task_result = self.task_pool.fetch_uncomplte(self.worker_id.clone()).await;
        if let Ok(un_complete_tasks) = un_complete_task_result {
            if un_complete_tasks.len() > 0 {
                let fetch_work = un_complete_tasks[0].clone();
                info!("worker {} fetch uncomplete task {}", self.worker_id, fetch_work.id);
                return Ok(fetch_work);
            }
        }
        self.task_pool.fetch_one_todo(self.worker_id.clone()).await
    }

    fn process_tasks(self){
        tokio::spawn(futures::future::lazy(async move|_|{
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
                    match self.fetch_one_todo().await {
                        Ok(undo_task) => {
                            count.fetch_add(1, Ordering::SeqCst);
                            let count_clone = count.clone();
                            let task_pool = self.task_pool.clone();
                            let worker_id = self.worker_id.clone();
                            let resource_result = self.resource.get_resource_info(undo_task.resource_id.clone()).await;
                            if let Err(e) = resource_result {
                                error!("unable to get resource of {}, reason:{}", undo_task.resource_id, e.to_string());
                                continue
                            }
                            let resource: Vec<u8> =  resource_result.unwrap().into();
                            tokio::spawn( futures::future::lazy(async move|_| {
                                defer! {
                                    count_clone.fetch_sub(1, Ordering::SeqCst);
                                }

                                if undo_task.task_type == TaskType::C2 {
                                    let c2: C2Resource = serde_json::from_slice(&resource).unwrap();
                                    info!("worker {} start to do task {}, size {}", worker_id.clone(), undo_task.id, u64::from(c2.phase1_output.registered_proof.sector_size()));
                                    match seal_commit_phase2(c2.phase1_output, c2.prove_id, c2.sector_id, ) {
                                        Ok(proof_arg) => {
                                            info!("worker {} complted {} success", worker_id.clone(), undo_task.id);
                                            let base64_proof = base64::encode(proof_arg.proof).to_string();
                                            task_pool.record_proof(worker_id.clone(), undo_task.id, base64_proof).await.unwrap();
                                        }
                                        Err(e) => {
                                            info!("worker {} execute {} fail reason {}", worker_id.clone(), undo_task.id, e.to_string());
                                            task_pool.record_error(worker_id.clone(), undo_task.id, e.to_string()).await.unwrap();
                                        }
                                    }
                                }

                            }));
                            
                        },
                        Err(e) => {
                            error!("unable to fetch undo task {}", e)
                        }
                    }
                }
            }));
        info!("worker has started");
    }
}