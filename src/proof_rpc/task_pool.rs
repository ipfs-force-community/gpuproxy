use std::ops::Deref;
use crate::models::{NewTask, Task, WorkerInfo, NewWorkerInfo};
use crate::models::schema::tasks::dsl::*;
use std::sync::{Mutex};
use diesel::insert_into;
use diesel::prelude::*;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use filecoin_proofs_api::seal::{SealCommitPhase1Output};

use anyhow::{anyhow, Result};
use chrono::Utc;
use log::info;
use uuid::Uuid;
use crate::models::schema::worker_infos::dsl::worker_infos;

#[derive(IntoPrimitive, TryFromPrimitive)]
#[repr(i32)]
pub enum TaskStatus {
    Undefined,
    Init,
    Running,
    Error,
    Completed,
}

pub trait WorkerApi {
    fn get_worker_id(&self) -> Result<uuid::Uuid>;
}

pub trait WorkerFetch {
    fn fetch_one_todo(&self,  worker_id_arg: String) -> Result<Task>;
    fn record_error(&self,  worker_id_arg: String, tid: i64,   err_msg: String) -> Option<anyhow::Error>;
    fn record_proof(&self,  worker_id_arg: String, tid: i64, proof: String) -> Option<anyhow::Error>;
}

pub trait Common {
    fn add(&self, miner_arg: forest_address::Address, prove_id_arg: String, sector_id_arg: i64,  phase1_output_arg: SealCommitPhase1Output) -> Result<i64>;
    fn fetch(&self, tid: i64) -> Result<Task>;
    fn fetch_undo(&self) -> Result<Vec<Task>>;
    fn get_status(&self, tid: i64) -> Result<TaskStatus>;
}

pub trait Taskpool:WorkerApi+WorkerFetch+Common{}
impl<T> Taskpool for T where T: WorkerApi + WorkerFetch + Common {}

pub struct TaskpoolImpl {
    conn: Mutex<SqliteConnection>,
}

impl TaskpoolImpl {
    pub fn new(conn: Mutex<SqliteConnection>) -> Self {
        TaskpoolImpl { conn }
    }
}

unsafe impl Send for TaskpoolImpl {}
unsafe impl Sync for TaskpoolImpl {}

impl WorkerApi for TaskpoolImpl {
    fn get_worker_id(&self) -> Result<uuid::Uuid> {
        let lock = self.conn.lock().map_err(|e|anyhow!(e.to_string()))?;
        let row_count: i64 =  worker_infos.count().get_result(lock.deref())?;
        if row_count == 0 {
           let uid =  uuid::Uuid::new_v4();
            let new_worker_info = NewWorkerInfo{
                worker_id: uid.to_string(),
            };
            let result = insert_into(worker_infos).values(&new_worker_info).execute(lock.deref())?;
            info!("create worker id {}", result);
           Ok(uid)
        } else {
            let worker_info: WorkerInfo =  worker_infos.first(lock.deref())?;
            let load_worker_id = Uuid::parse_str(worker_info.worker_id.as_str())?;
            info!("load worker id {}", load_worker_id.to_string());
            Ok(load_worker_id)
        }
    }
}

impl WorkerFetch for TaskpoolImpl {
    fn fetch_one_todo(&self, worker_id_arg: String) -> Result<Task> {
        let lock = self.conn.lock().map_err(|e|anyhow!(e.to_string()))?;
        let predicate = status.eq::<i32>(TaskStatus::Init.into());
        let result: Task = tasks.filter(&predicate).first(lock.deref())?;
        let update_result = diesel::update(tasks.filter(id.eq(result.id))).set((
            status.eq::<i32>(TaskStatus::Running.into()),
            worker_id.eq(worker_id_arg.clone()),
            start_at.eq(Utc::now().timestamp()),
        )).execute(lock.deref());
        info!("worker {} fetch {} to do", worker_id_arg, result.id);
        match update_result {
            Ok(_) => Ok(result),
            Err(e) => Err(anyhow!(e.to_string())),
        }
    }

    fn record_error(&self,  worker_id_arg: String, tid: i64, err_msg_str: String) -> Option<anyhow::Error> {
        let lock_result = self.conn.lock();
        if let Some(e) = lock_result.as_ref().err() {return Some(anyhow!(e.to_string()));}
        let lock = lock_result.unwrap();
        let update_result = diesel::update(
            tasks.filter(
                id.eq(tid)
            )
        ).set(
            (
                    status.eq::<i32>(TaskStatus::Error.into()),
                    worker_id.eq(worker_id_arg.clone()),
                    error_msg.eq(err_msg_str.clone()),
                   )
            ).execute(lock.deref());
        info!("worker {} mark task {} as error reason:{}", worker_id_arg, tid, err_msg_str);
        match update_result {
            Ok(_) => Option::None,
            Err(e) => Some(anyhow!(e.to_string())),
        }
    }

    fn record_proof(&self,  worker_id_arg: String, tid: i64, proof_str: String) -> Option<anyhow::Error> {
        let lock_result = self.conn.lock();
        if let Some(e) = lock_result.as_ref().err() {return Some(anyhow!(e.to_string()));}
        let lock = lock_result.unwrap();

        let update_result = diesel::update(
            tasks.filter(
                id.eq(tid)
                    )
        ).set(
            (status.eq::<i32>(TaskStatus::Completed.into()),
                         worker_id.eq(worker_id_arg.clone()),
                         proof.eq(proof_str),
                    )
        ).execute(lock.deref());
        info!("worker {} complete task {} successfully", worker_id_arg, tid);
        match update_result {
            Ok(_) => Option::None,
            Err(e) => Some(anyhow!(e.to_string())),
        }
    }
}

impl Common for TaskpoolImpl {
    fn add(&self, miner_arg: forest_address::Address, prove_id_arg: String, sector_id_arg: i64,  phase1_output_arg: SealCommitPhase1Output,) -> Result<i64> {
        let miner_noprefix = &miner_arg.to_string()[1..];
        let new_task = NewTask{
            miner: miner_noprefix.to_string(),
            worker_id: "".to_string(),
            prove_id: prove_id_arg,
            sector_id: sector_id_arg,
            phase1_output: serde_json::to_string(&phase1_output_arg)?,
            task_type:0,
            status:TaskStatus::Init.into(),
            create_at: Utc::now().timestamp(),
        };

        let lock = self.conn.lock().map_err(|e|anyhow!(e.to_string()))?;
        let result = insert_into(tasks).values(&new_task).execute(lock.deref());
        match result {
            Ok(val) => {
                info!("add task {} from miner {} sector {}", val, miner_arg, sector_id_arg);
                Ok(val as i64)
            },
            Err(e) => Err(anyhow!(e.to_string())),
        }
    }

    fn fetch(&self, tid: i64) -> Result<Task> {
        let lock = self.conn.lock().map_err(|e|anyhow!(e.to_string()))?;
        let result = tasks.find(tid).first(lock.deref());
        match result {
            Ok(val) => Ok(val),
            Err(e) => Err(anyhow!(e.to_string())),
        }
    }

    fn fetch_undo(&self) -> Result<Vec<Task>> {
        let lock = self.conn.lock().map_err(|e|anyhow!(e.to_string()))?;
        let result = tasks.filter(status.eq::<i32>(TaskStatus::Init.into()))
            .load(lock.deref());
        match result {
            Ok(val) => Ok(val),
            Err(e) => Err(anyhow!(e.to_string())),
        }
    }

    fn get_status(&self, tid: i64) -> Result<TaskStatus> {
        let lock = self.conn.lock().map_err(|e|anyhow!(e.to_string()))?;
        let result: QueryResult::<i32> = tasks.select(status).filter(id.eq(tid)).get_result(lock.deref());
        match result {
            Ok(val) => Ok(TaskStatus::try_from(val)?),
            Err(e) => Err(anyhow!(e.to_string())),
        }
    }
}

#[cfg(test)]
mod tests{

    #[test]
    pub fn test_status() {

    }
}
