use crate::proof_rpc::db_ops::*;
use crate::resource::{C2Resource, Resource};
use anyhow::{anyhow, Result};
use crossbeam_channel::tick;
use filecoin_proofs_api::seal::seal_commit_phase2;
use log::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::oneshot;

use async_trait::async_trait;
use entity::resource_info as ResourceInfos;
use entity::tasks as Tasks;
use entity::tasks::TaskType;
use entity::worker_info as WorkerInfos;
use tokio::time::{sleep, Duration};
use ResourceInfos::Model as ResourceInfo;
use Tasks::Model as Task;
use WorkerInfos::Model as WorkerInfo;
use crate::utils::*;

#[async_trait]
pub trait Worker {
    async fn process_tasks(self);
}

pub struct LocalWorker {
    pub worker_id: String,
    pub max_task: usize,
    pub task_fetcher: Arc<dyn WorkerFetch + Send + Sync>,
    pub resource: Arc<dyn Resource + Send + Sync>,
}

impl LocalWorker {
    pub fn new(
        max_task: usize,
        worker_id: String,
        resource: Arc<dyn Resource + Send + Sync>,
        task_fetcher: Arc<dyn WorkerFetch + Send + Sync>,
    ) -> Self {
        LocalWorker {
            worker_id,
            max_task,
            task_fetcher,
            resource,
        }
    }
}

#[async_trait]
impl Worker for LocalWorker {
    async fn process_tasks(self) {
        let (tx, mut rx) = channel(5);
        let fetcher = Arc::new(self.task_fetcher);
        let count = Arc::new(AtomicUsize::new(0));

        {
            let worker_id = self.worker_id.clone();
            let fetcher = fetcher.clone();
            let count_clone = count.clone();
            tokio::spawn(
                futures::future::lazy(async move |_| {
                    info!("start task fetcher, wait for new task todo");
                    let mut un_complete_task_result = fetcher.fetch_uncomplte(worker_id.to_string()).await.unwrap();
                    let ticker = tick(Duration::from_secs(10));
                    loop {
                        ticker.recv().unwrap();
                        let cur_size = count_clone.load(Ordering::SeqCst);
                        if cur_size >= self.max_task {
                            warn!("has reach the max number of c2 tasks {} {}", cur_size, self.max_task);
                            continue;
                        }

                        let select_task:Task;
                        if un_complete_task_result.len() > 0 {
                            select_task = un_complete_task_result.pop().unwrap();
                        }else{
                            match fetcher.fetch_one_todo(worker_id.clone()).await {
                                Ok(v) => {
                                    select_task  = v;
                                }
                                Err(e) => {
                                    error!("unable to get task {}", e);
                                    continue;
                                },
                            }
                        }

                        let task_id = select_task.id.clone();
                        if let Err(e) = tx.send(select_task).await {
                            error!("unable to send task to channel {:?}", e);
                        }else{
                            debug!("send new task {} to channel", task_id);
                            count_clone.fetch_add(1, Ordering::SeqCst);
                        }
                    }
                })
                .await,
            );
        }

        {
            let worker_id = self.worker_id.clone();
            let fetcher = fetcher.clone();
            let count_clone = count.clone();
            let (result_tx, mut result_rx) = channel(1);
            tokio::spawn(
                futures::future::lazy(async move |_| {
                    info!("worker {} start to worker and wait for new tasks", worker_id.clone());
                    loop {
                        tokio::select! {
                            undo_task_result = rx.recv() => {
                                   debug!("receive task from channel",);
                                   match undo_task_result {
                                        Some(undo_task) => {
                                            let task_id = undo_task.id.clone();
                                            let resource_id = undo_task.resource_id.clone();

                                            let resource_result = self.resource.get_resource_info(resource_id.clone()).await;
                                            if let Err(e) = resource_result {
                                                error!("unable to get resource of {}, reason:{}", resource_id.clone(), e.to_string());
                                                count_clone.fetch_sub(1, Ordering::SeqCst);
                                                continue;
                                            }
                                            let resource: Vec<u8> = resource_result.unwrap().into();

                                            let worker_id = worker_id.clone();
                                            let result_tx_clone = result_tx.clone();
                                            info!("prepare successfully for task {} and spawn to run", undo_task.id);
                                            std::thread::spawn( //avoid block schedule
                                                 move || {
                                                    let result = if undo_task.task_type == TaskType::C2 {
                                                        //todo ensure send error result for each error condition
                                                        serde_json::from_slice(&resource).anyhow()
                                                            .and_then(|c2:C2Resource|{
                                                                info!("worker {} start to do task {}, size {}", worker_id, undo_task.id, u64::from(c2.c1out.registered_proof.sector_size()));
                                                                seal_commit_phase2(c2.c1out, c2.prover_id, c2.sector_id)
                                                        })
                                                    }else{
                                                       Err(anyhow!("unsupport type of task {} type {:?}", undo_task.id, undo_task.task_type))
                                                    };
                                                    futures::executor::block_on(result_tx_clone.send((undo_task, result))).unwrap();
                                                },
                                            );
                                        }
                                        None => {
                                            error!("unable to fetch undo task, should never occur");
                                            sleep(Duration::from_millis(100)).await;
                                        }
                                    }
                            }
                            val = result_rx.recv() => {
                                   debug!("receive excute result from channel");
                                   defer! {
                                            count_clone.fetch_sub(1, Ordering::SeqCst);
                                   }
                                   if let Some((undo_task, exec_result)) = val {
                                        match exec_result {
                                            Ok(proof_arg) => {
                                                info!("worker {} complted {} success", worker_id.clone(), undo_task.id);
                                                let base64_proof = base64::encode(proof_arg.proof).to_string();
                                                if let Some(e) = fetcher.record_proof(worker_id.clone(), undo_task.id.clone(), base64_proof).await{
                                                    error!("record proof for task {} error reason {}", undo_task.id.clone(), e.to_string())
                                                }
                                            }
                                            Err(e) => {
                                                info!(
                                                    "worker {} execute {} fail reason {}",
                                                    worker_id.clone(),
                                                    undo_task.id,
                                                    e.to_string()
                                                );
                                               if let Some(e) = fetcher.record_error(worker_id.clone(), undo_task.id.clone(), e.to_string()).await{
                                                   error!("record error for task {} error reason {}", undo_task.id.clone(), e.to_string())
                                               }
                                            }
                                        }
                                }
                            }
                            _ = sleep(Duration::from_secs(10)) =>{info!("wait for new task or execute result")}
                        }
                    }
                })
                .await,
            );
        }
        info!("worker has started");
    }
}
