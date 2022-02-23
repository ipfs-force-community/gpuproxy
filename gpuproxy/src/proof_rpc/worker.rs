use crate::proof_rpc::db_ops::*;
use crate::resource::{C2Resource, Resource};
use anyhow::Result;
use crossbeam_channel::tick;
use filecoin_proofs_api::seal::seal_commit_phase2;
use log::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, Sender, SyncSender};
use std::sync::{mpsc, Arc};

use async_trait::async_trait;
use entity::resource_info as ResourceInfos;
use entity::tasks as Tasks;
use entity::tasks::TaskType;
use entity::worker_info as WorkerInfos;
use ResourceInfos::Model as ResourceInfo;
use Tasks::Model as Task;
use WorkerInfos::Model as WorkerInfo;
use tokio::time::{sleep, Duration};

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
        let (tx, rx): (SyncSender<Task>, Receiver<Task>) = mpsc::sync_channel(0);
        let count = Arc::new(AtomicUsize::new(0));
        let fetcher = Arc::new(self.task_fetcher);

        {
            let count_clone = count.clone();
            let worker_id = self.worker_id.clone();
            let fetcher = fetcher.clone();
            tokio::spawn(
                futures::future::lazy(async move |_| {
                    info!("start task fetcher, wait for new task todo");
                    let mut un_complete_task_result = fetcher.fetch_uncomplte(worker_id.to_string()).await.unwrap();
                    let ticker = tick(Duration::from_secs(10));
                    loop {
                        ticker.recv().unwrap();
                        let cur_size = count_clone.clone().load(Ordering::SeqCst);
                        if cur_size >= self.max_task {
                            warn!("has reach the max number of c2 tasks {} {}", cur_size, self.max_task);
                            continue;
                        }

                        if un_complete_task_result.len() > 0 {
                            if let Err(e) = tx.send(un_complete_task_result.pop().unwrap()) {
                                error!("unable to send task to channel {}", e);
                            }
                            continue;
                        }

                        if let Err(e) = fetcher.fetch_one_todo(worker_id.clone()).await.map(|v| tx.send(v)) {
                            error!("unable to get task {}", e);
                        }
                    }
                })
                .await,
            );
        }

        {
            let count_clone = count.clone();
            let worker_id = self.worker_id.clone();
            let fetcher = fetcher.clone();
            tokio::spawn(
                futures::future::lazy(async move |_| {
                    info!("worker {} start to worker and wait for new tasks", worker_id.clone());
                    loop {
                        let undo_task_result = rx.recv();
                        match undo_task_result {
                            Ok(undo_task) => {
                                let resource_result = self.resource.get_resource_info(undo_task.resource_id.clone()).await;
                                if let Err(e) = resource_result {
                                    error!("unable to get resource of {}, reason:{}", undo_task.resource_id, e.to_string());
                                    continue;
                                }
                                let resource: Vec<u8> = resource_result.unwrap().into();

                                count_clone.fetch_add(1, Ordering::SeqCst);
                                let count_clone2 = count_clone.clone();
                                let task_recorder = fetcher.clone();
                                let worker_id = worker_id.clone();

                                info!("prepare successfully for task {} and spawn to run", undo_task.id);
                                tokio::spawn(
                                    futures::future::lazy(async move |_| {
                                        info!("start finishsadasdasdasdas");
                                        defer! {
                                            count_clone2.fetch_sub(1, Ordering::SeqCst);
                                        }
                                        info!("start finishsadasdasdasdas");
                                        if undo_task.task_type == TaskType::C2 {
                                            let c2: C2Resource = serde_json::from_slice(&resource).unwrap();
                                            info!(
                                                "worker {} start to do task {}, size {}",
                                                worker_id.clone(),
                                                undo_task.id,
                                                u64::from(c2.phase1_output.registered_proof.sector_size())
                                            );
                                            match seal_commit_phase2(c2.phase1_output, c2.prove_id, c2.sector_id) {
                                                Ok(proof_arg) => {
                                                    info!("worker {} complted {} success", worker_id.clone(), undo_task.id);
                                                    let base64_proof = base64::encode(proof_arg.proof).to_string();
                                                    task_recorder
                                                        .record_proof(worker_id.clone(), undo_task.id, base64_proof)
                                                        .await
                                                        .unwrap();
                                                }
                                                Err(e) => {
                                                    info!(
                                                        "worker {} execute {} fail reason {}",
                                                        worker_id.clone(),
                                                        undo_task.id,
                                                        e.to_string()
                                                    );
                                                    task_recorder
                                                        .record_error(worker_id.clone(), undo_task.id, e.to_string())
                                                        .await
                                                        .unwrap();
                                                }
                                            }
                                        }
                                    })
                                    .await,
                                );
                            }
                            Err(e) => {
                                error!("unable to fetch undo task {}", e);
                                sleep(Duration::from_millis(100)).await;
                            }
                        }
                    }
                })
                .await,
            );
        }
        info!("worker has started");
    }
}
