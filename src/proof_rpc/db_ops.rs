use std::ops::Deref;
use std::sync::{Mutex};

use entity::tasks as Tasks;
use entity::resource_info as ResourceInfos;
use entity::worker_info as WorkerInfos;
use Tasks::Model as Task;
use ResourceInfos::Model as ResourceInfo;
use WorkerInfos::Model as WorkerInfo;

use sea_orm::DatabaseConnection;
use anyhow::{anyhow, Result};
use chrono::Utc;
use entity::tasks::TaskStatus;
use log::info;
use sea_orm::ActiveValue::Set;
use uuid::Uuid;
use crate::resource::Resource;
use crate::utils::{*};
use crate::utils::base64bytes::Base64Byte;
use sea_orm::entity::prelude::*;


pub trait WorkerApi {
    fn get_worker_id(&self) -> Result<uuid::Uuid>;
}

pub trait WorkerFetch {
    fn fetch_one_todo(&self,  worker_id_arg: String) -> Result<Task>;
    fn fetch_uncomplte(&self, worker_id_arg: String) -> Result<Vec<Task>>;
    fn record_error(&self,  worker_id_arg: String, tid: String,   err_msg: String) -> Option<anyhow::Error>;
    fn record_proof(&self,  worker_id_arg: String, tid: String, proof: String) -> Option<anyhow::Error>;
}

pub trait Common {
    fn add_task(&self, miner_arg: forest_address::Address, resource_id: String) -> Result<String>;
    fn fetch(&self, tid: String) -> Result<Task>;
    fn fetch_undo(&self) -> Result<Vec<Task>>;
    fn get_status(&self, tid: String) -> Result<TaskStatus>;
    fn list_task(&self, worker_id_arg: Option<String>, state: Option<Vec<i32>>) -> Result<Vec<Task>>;
}

pub trait DbOp:WorkerApi+WorkerFetch+Common{}
impl<T> DbOp for T where T: WorkerApi + WorkerFetch + Common {}

pub struct TaskpoolImpl {
    rt : tokio::runtime::Runtime,
    conn: Mutex<DatabaseConnection>,
}

impl TaskpoolImpl {
    pub fn new(conn: Mutex<DatabaseConnection>) -> Self {
        let rt = tokio::runtime::Runtime::new().unwrap();
        TaskpoolImpl {rt,conn }
    }
}

unsafe impl Send for TaskpoolImpl {}
unsafe impl Sync for TaskpoolImpl {}

impl WorkerApi for TaskpoolImpl {
    fn get_worker_id(&self) -> Result<uuid::Uuid> {
        let db = self.conn.lock().map_err(|e|anyhow!(e.to_string()))?;
        let worker_info_op: Option<WorkerInfo> = self.rt.block_on(WorkerInfos::Entity::find().one(db.deref()))?;

        if let Some(worker_info) = worker_info_op {
            let load_worker_id = Uuid::parse_str(worker_info.id.as_str())?;
            info!("load worker id {}", load_worker_id.to_string());
            Ok(load_worker_id)

        } else {
            let uid =  uuid::Uuid::new_v4();
            let new_worker_info = WorkerInfos::ActiveModel{
                id: Set(uid.to_string()),
            };
            let _ = self.rt.block_on(new_worker_info.save(db.deref()))?;
            info!("create worker id {}", uid);
            Ok(uid)
        }
    }
}

impl WorkerFetch for TaskpoolImpl {
    fn fetch_one_todo(&self, worker_id_arg: String) -> Result<Task> {
        let lock = self.conn.lock().map_err(|e|anyhow!(e.to_string()))?;
        let result: Task = tasks_dsl::tasks.filter(tasks_dsl::status.eq::<i32>(TaskStatus::Init.into())).first(lock.deref())?;
        diesel::update(tasks_dsl::tasks.filter(tasks_dsl::id.eq(result.id.clone()))).set((
            tasks_dsl::status.eq::<i32>(TaskStatus::Running.into()),
            tasks_dsl::worker_id.eq(worker_id_arg.clone()),
            tasks_dsl:: start_at.eq(Utc::now().timestamp()),
        )).execute(lock.deref()).map(|_|{
            info!("worker {} fetch {} to do", worker_id_arg, result.id);
            result}).anyhow()
    }

    fn fetch_uncomplte(&self, worker_id_arg: String) -> Result<Vec<Task>>{
        todo!()
    }

    fn record_error(&self,  worker_id_arg: String, tid: String, err_msg_str: String) -> Option<anyhow::Error> {
        todo!()
    }

    fn record_proof(&self,  worker_id_arg: String, tid: String, proof_str: String) -> Option<anyhow::Error> {
        todo!()
    }
}

impl Common for TaskpoolImpl {
    fn add_task(&self, miner_arg: forest_address::Address, resource_id: String) -> Result<String> {
        todo!()
    }

    fn fetch(&self, tid: String) -> Result<Task> {
        todo!()
    }

    fn fetch_undo(&self) -> Result<Vec<Task>> {
        todo!()
    }

    fn get_status(&self, tid: String) -> Result<TaskStatus> {
        todo!()
    }
    fn list_task(&self, worker_id_opt: Option<String>, state_cod: Option<Vec<i32>>) -> Result<Vec<Task>> {
        todo!()
    }
}

impl Resource for TaskpoolImpl {
    fn get_resource_info(&self, resource_id: String) -> Result<Base64Byte> {
        todo!()
    }

    fn store_resource_info(&self, resource: Vec<u8>) -> Result<String> {
        todo!()
    }
}


#[cfg(test)]
mod tests{

    #[test]
    pub fn test_status() {

    }
}
