use std::error::Error;
use std::ops::Deref;
use crate::models::{NewTask, Task};
use crate::models::schema::tasks::dsl::*;
use std::sync::{Mutex};
use diesel::insert_into;
use diesel::prelude::*;
use jsonrpc_core::to_string;
use num_enum::{IntoPrimitive, TryFromPrimitive};

use std::time::Duration;
use log::error;
use ticker::Ticker;

use log::*;
use simplelog::*;
use anyhow::{anyhow, Result};
use chrono::Utc;

#[derive(IntoPrimitive, TryFromPrimitive)]
#[repr(i32)]
pub enum TaskStatus {
    Undefined,
    Init,
    Running,
    Error,
    Completed,
}

pub trait Taskpool {
    fn add(&self, miner_arg: String, prove_id_arg: String, sector_id_arg: i64,  phase1_output_arg: String,) -> Result<i64>;
    fn fetch(&self, tid: i64) -> Result<Task>;
    fn fetch_undo(&self) -> Result<Vec<Task>>;
    fn fetch_one_todo(&self) -> Result<Task>;
    fn get_status(&self, tid: i64) -> Result<TaskStatus>;
    fn record_error(&self, tid: i64, err_msg: String) -> Option<anyhow::Error>;
    fn record_proof(&self, tid: i64, proof: String) -> Option<anyhow::Error>;
}

pub struct TaskpoolImpl {
    conn: Mutex<SqliteConnection>,
}

impl Taskpool for TaskpoolImpl {
    fn add(&self, miner_arg: String, prove_id_arg: String, sector_id_arg: i64,  phase1_output_arg: String,) -> Result<i64> {
        let new_task = NewTask{
            miner: miner_arg,
            prove_id: prove_id_arg,
            sector_id: sector_id_arg,
            phase1_output: phase1_output_arg,
            task_type:0,
            status:TaskStatus::Init.into(),
            create_at: Utc::now().timestamp(),
        };
        let lock = self.conn.lock().unwrap();
        let result = insert_into(tasks).values(&new_task).execute(lock.deref());

        match result {
            Ok(val) => Ok(val as i64),
            Err(e) => Err(anyhow!(e.to_string())),
        }
    }

    fn fetch(&self, tid: i64) -> Result<Task> {
        let lock = self.conn.lock().unwrap();
        let result = tasks.find(tid).first(lock.deref());
        match result {
            Ok(val) => Ok(val),
            Err(e) => Err(anyhow!(e.to_string())),
        }
    }

    fn get_status(&self, tid: i64) -> Result<TaskStatus> {
        let lock = self.conn.lock().unwrap();
        let result: QueryResult::<i32> = tasks.select(status).filter(id.eq(tid)).get_result(lock.deref());
        match result {
            Ok(val) => Ok(TaskStatus::try_from(val).unwrap()),
            Err(e) => Err(anyhow!(e.to_string())),
        }
    }

    fn fetch_undo(&self) -> Result<Vec<Task>> {
        let lock = self.conn.lock().unwrap();
        let result = tasks.filter(status.eq::<i32>(TaskStatus::Init.into()))
            .load(lock.deref());
        match result {
            Ok(val) => Ok(val),
            Err(e) => Err(anyhow!(e.to_string())),
        }
    }

    fn fetch_one_todo(&self) -> Result<Task> {
        let lock = self.conn.lock().unwrap();
        let predicate = status.eq::<i32>(TaskStatus::Init.into());
        let result: Task = tasks.filter(&predicate).first(lock.deref()).unwrap();
        let update_result = diesel::update(tasks.filter(id.eq(result.id))).set((
            status.eq::<i32>(TaskStatus::Init.into()),
            start_at.eq(Utc::now().timestamp()),
        )).execute(lock.deref());
        match update_result {
            Ok(val) => Ok(result),
            Err(e) => Err(anyhow!(e.to_string())),
        }
    }

    fn record_error(&self, _tid: i64, err_msg_str: String) -> Option<anyhow::Error> {
        let lock = self.conn.lock().unwrap();
        let update_result = diesel::update(tasks.filter(id.eq(_tid))).set((
                                                           status.eq::<i32>(TaskStatus::Error.into()),
                                                           error_msg.eq(err_msg_str),
                                                           )).execute(lock.deref());
        match update_result {
            Ok(val) => Option::None,
            Err(e) => Some(anyhow!(e.to_string())),
        }
    }

    fn record_proof(&self, _tid: i64, proof_str: String) -> Option<anyhow::Error> {
        let lock = self.conn.lock().unwrap();
        let update_result = diesel::update(tasks.filter(id.eq(_tid))).set((
            status.eq::<i32>(TaskStatus::Error.into()),
            proof.eq(proof_str),
        )).execute(lock.deref());
        match update_result {
            Ok(_) => Option::None,
            Err(e) => Some(anyhow!(e.to_string())),
        }
    }
}




#[cfg(test)]
mod tests{

    #[test]
    pub fn test_status() {

    }
}
