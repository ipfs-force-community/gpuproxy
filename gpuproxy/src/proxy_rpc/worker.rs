use crate::proxy_rpc::db_ops::*;
use crate::resource::{C2Resource, Resource};
use crate::utils::*;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use entity::resource_info as ResourceInfos;
use entity::tasks as Tasks;
use entity::worker_info as WorkerInfos;
use entity::TaskType;
use filecoin_proofs_api::seal::{
    seal_commit_phase2, SealCommitPhase1Output, SealCommitPhase2Output,
};
use log::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::time;
use tokio::time::{sleep, Duration};
use ResourceInfos::Model as ResourceInfo;
use Tasks::Model as Task;
use WorkerInfos::Model as WorkerInfo;

/// GPU worker used to execute specify task
#[async_trait]
pub trait Worker {
    async fn process_tasks(self);
}

/// Local worker to execute gpu task in local machine
pub struct LocalWorker {
    pub worker_id: String,
    pub max_task: usize,
    pub task_fetcher: Arc<dyn WorkerFetch + Send + Sync>,
    pub resource: Arc<dyn Resource + Send + Sync>,
    pub allow_types: Option<Vec<TaskType>>,
}

impl LocalWorker {
    pub fn new(
        max_task: usize,
        worker_id: String,
        allow_types: Option<Vec<TaskType>>,
        resource: Arc<dyn Resource + Send + Sync>,
        task_fetcher: Arc<dyn WorkerFetch + Send + Sync>,
    ) -> Self {
        LocalWorker {
            worker_id,
            max_task,
            task_fetcher,
            resource,
            allow_types,
        }
    }
}

#[async_trait]
impl Worker for LocalWorker {
    async fn process_tasks(self) {
        let (tx, mut rx) = channel(self.max_task);
        let fetcher = Arc::new(self.task_fetcher);
        let count = Arc::new(AtomicUsize::new(0));

        {
            let worker_id = self.worker_id.clone();
            let fetcher = fetcher.clone();
            let count_clone = count.clone();
            tokio::spawn(async move {
                info!("start task fetcher, wait for new task todo");
                let mut un_complete_task_result = fetcher
                    .fetch_uncompleted(worker_id.to_string())
                    .await
                    .expect(
                        "unable to get not completed task, check gpuproxy server config correct ",
                    );
                let mut interval = time::interval(Duration::from_secs(10));
                loop {
                    interval.tick().await;
                    let cur_size = count_clone.load(Ordering::SeqCst);
                    if cur_size >= self.max_task {
                        warn!(
                            "has reach the max number of c2 tasks {} {}",
                            cur_size, self.max_task
                        );
                        continue;
                    }

                    let select_task: Task;
                    if !un_complete_task_result.is_empty() {
                        select_task = un_complete_task_result.pop().unwrap();
                    } else {
                        match fetcher
                            .fetch_one_todo(worker_id.clone(), self.allow_types.clone())
                            .await
                        {
                            Ok(v) => {
                                select_task = v;
                            }
                            Err(e) => {
                                debug!("unable to get task {}", e);
                                continue;
                            }
                        }
                    }

                    let task_id = select_task.id.clone();
                    if let Err(e) = tx.send(select_task).await {
                        error!("unable to send task to channel {:?}", e);
                    } else {
                        debug!("send new task {} to channel", task_id);
                        count_clone.fetch_add(1, Ordering::SeqCst);
                    }
                }
            });
        }

        {
            let worker_id = self.worker_id.clone();
            let fetcher = fetcher.clone();
            let (result_tx, mut result_rx) = channel(1);
            tokio::spawn(async move {
                info!(
                    "worker {} start to worker and wait for new tasks",
                    worker_id.clone()
                );
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
                                            count.fetch_sub(1, Ordering::SeqCst);
                                            continue;
                                        }
                                        let resource: Vec<u8> = resource_result.unwrap().into();

                                        let result_tx_clone = result_tx.clone();
                                        info!("worker {} prepare successfully for task {} and spawn to run", worker_id, undo_task.id);
                                        tokio::spawn(async move {
                                            let result = if undo_task.task_type == TaskType::C2 {
                                                c2(resource).await
                                            } else {
                                                Err(anyhow!("unsupported type of task {} type {:?}", undo_task.id, undo_task.task_type))
                                            };
                                            let _ = result_tx_clone.send((undo_task, result)).await;
                                        });
                                    }
                                    None => {
                                        error!("unable to fetch undo task, should never occur");
                                        sleep(Duration::from_secs(60)).await;
                                    }
                                }
                        }
                        val = result_rx.recv() => {
                               debug!("receive excute result from channel");
                               defer! {
                                        count.fetch_sub(1, Ordering::SeqCst);
                               }
                               if let Some((undo_task, exec_result)) = val {
                                    match exec_result {
                                        Ok(proof_arg) => {
                                            info!("worker {} completed {} success", worker_id.clone(), undo_task.id);
                                            if let Some(e) = fetcher.record_proof(worker_id.clone(), undo_task.id.clone(), proof_arg.proof).await{
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
                        _ = sleep(Duration::from_secs(10)) =>{debug!("wait for new task or execute result")}
                    }
                }
            });
        }
        info!("worker has started");
    }
}

async fn c2(resource: Vec<u8>) -> anyhow::Result<SealCommitPhase2Output> {
    let join_result = tokio::task::spawn_blocking(move || {
        let c2_resource: C2Resource =
            serde_json::from_slice(&resource).context("deserialize c2 resource")?;
        info!(
            "start to do c2 task. size: {}",
            u64::from(c2_resource.c1out.registered_proof.sector_size())
        );

        seal_commit_phase2(
            c2_resource.c1out,
            c2_resource.prover_id,
            c2_resource.sector_id,
        )
    })
    .await;

    match join_result {
        Ok(task_result) => task_result,
        Err(join_error) => {
            if join_error.is_panic() {
                let panic_error = join_error.into_panic();
                let message = match panic_error.downcast_ref::<&str>() {
                    Some(msg) => msg.to_string(),
                    None => panic_error
                        .downcast_ref::<String>()
                        .cloned()
                        .unwrap_or_else(|| "non string panic payload".to_string()),
                };
                Err(anyhow::Error::msg(format!("Panic: {}", message)))
            } else {
                Err(anyhow::Error::msg(join_error.to_string()))
            }
        }
    }
}
