use std::ops::Deref;
use crate::models::{NewTask, Task, WorkerInfo, ResourceInfo, NewWorkerInfo, TaskStatus, TaskType};
use crate::models::schema::tasks::dsl as tasks_dsl;
use crate::models::schema::worker_infos::dsl as worker_infos_dsl;
use crate::models::schema::resource_infos::dsl as resource_infos_dsl;
use crate::proof_rpc::resource::{*};
use std::sync::{Mutex};
use diesel::insert_into;
use diesel::prelude::*;

use anyhow::{anyhow, Result};
use chrono::Utc;
use forest_address::Error::Base32Decoding;
use log::info;
use uuid::Uuid;
use crate::proof_rpc::utils::IntoAnyhow;
use crate::models::Bas64Byte;

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
        let row_count: i64 =  worker_infos_dsl::worker_infos.count().get_result(lock.deref())?;
        if row_count == 0 {
           let uid =  uuid::Uuid::new_v4();
            let new_worker_info = NewWorkerInfo{
                id: uid.to_string(),
            };
            let result = insert_into(worker_infos_dsl::worker_infos).values(&new_worker_info).execute(lock.deref())?;
            info!("create worker id {}", result);
           Ok(uid)
        } else {
            let worker_info: WorkerInfo = worker_infos_dsl::worker_infos.first(lock.deref())?;
            let load_worker_id = Uuid::parse_str(worker_info.id.as_str())?;
            info!("load worker id {}", load_worker_id.to_string());
            Ok(load_worker_id)
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
        let lock = self.conn.lock().map_err(|e|anyhow!(e.to_string()))?;
        tasks_dsl::tasks.filter(
            tasks_dsl::worker_id.eq(worker_id_arg).and(
                tasks_dsl::status.eq::<i32>(TaskStatus::Running.into())
            )
        )
            .load(lock.deref()).anyhow()
    }

    fn record_error(&self,  worker_id_arg: String, tid: String, err_msg_str: String) -> Option<anyhow::Error> {
        let lock_result = self.conn.lock();
        if let Some(e) = lock_result.as_ref().err() {return Some(anyhow!(e.to_string()));}
        let lock = lock_result.unwrap();
        diesel::update(
            tasks_dsl::tasks.filter(
                tasks_dsl::id.eq(tid.clone())
            )
        ).set(
            (
                tasks_dsl::status.eq::<i32>(TaskStatus::Error.into()),
                tasks_dsl::worker_id.eq(worker_id_arg.clone()),
                tasks_dsl::error_msg.eq(err_msg_str.clone()),
                   )
            ).execute(lock.deref()).map(|e|{
            info!("worker {} mark task {} as error reason:{}", worker_id_arg, tid, err_msg_str);
            e
        }).err().map(|e|anyhow!(e.to_string()))
    }

    fn record_proof(&self,  worker_id_arg: String, tid: String, proof_str: String) -> Option<anyhow::Error> {
        let lock_result = self.conn.lock();
        if let Some(e) = lock_result.as_ref().err() {return Some(anyhow!(e.to_string()));}
        let lock = lock_result.unwrap();

        diesel::update(
            tasks_dsl::tasks.filter(
                tasks_dsl::id.eq(tid.clone())
                    )
        ).set(
            (tasks_dsl::status.eq::<i32>(TaskStatus::Completed.into()),
             tasks_dsl::worker_id.eq(worker_id_arg.clone()),
             tasks_dsl::proof.eq(proof_str),
                tasks_dsl::create_at.eq(Utc::now().timestamp())
                    )
        ).execute(lock.deref())
            .map(|e|{
                info!("worker {} complete task {} successfully", worker_id_arg, tid);
                e
            }).err().map(|e|anyhow!(e.to_string()))
    }
}

impl Common for TaskpoolImpl {
    fn add_task(&self, miner_arg: forest_address::Address, resource_id: String) -> Result<String> {
        let miner_noprefix = &miner_arg.to_string()[1..];
        let new_task_id =  Uuid::new_v4().to_string();
        let new_task = NewTask{
            id: new_task_id.clone(),
            miner: miner_noprefix.to_string(),
            resource_id: resource_id.clone(),
            worker_id: "".to_string(),
            task_type: TaskType::C2,
            status:TaskStatus::Init.into(),
            create_at: Utc::now().timestamp(),
        };

        let lock = self.conn.lock().map_err(|e|anyhow!(e.to_string()))?;
        insert_into(tasks_dsl::tasks).values(&new_task)
            .execute(lock.deref())
            .anyhow()
            .and(Ok(new_task_id))
    }

    fn fetch(&self, tid: String) -> Result<Task> {
        let lock = self.conn.lock().map_err(|e|anyhow!(e.to_string()))?;
        tasks_dsl::tasks.find(tid).first(lock.deref()).anyhow()
    }

    fn fetch_undo(&self) -> Result<Vec<Task>> {
        let lock = self.conn.lock().map_err(|e|anyhow!(e.to_string()))?;
         tasks_dsl::tasks.filter(tasks_dsl::status.eq::<i32>(TaskStatus::Init.into()))
            .load(lock.deref())
             .anyhow()
    }

    fn get_status(&self, tid: String) -> Result<TaskStatus> {
        let lock = self.conn.lock().map_err(|e|anyhow!(e.to_string()))?;
       tasks_dsl::tasks.select(tasks_dsl::status)
            .filter(tasks_dsl::id.eq(tid))
            .get_result(lock.deref())
          //.map(|val: i32|TaskStatus::try_from(val)?) cannot compile ?
            .map_err(|e|anyhow!(e.to_string()))
            .map(|val: i32|TaskStatus::try_from(val).map_err(|e|anyhow!(e.to_string()))) //todo change unwrap to ?
            .flatten()
            .anyhow()
    }
    fn list_task(&self, worker_id_opt: Option<String>, state_cod: Option<Vec<i32>>) -> Result<Vec<Task>> {
        let mut query =  tasks_dsl::tasks.into_boxed();
        if let Some(worker_id_arg) = worker_id_opt {
            query = query.filter(tasks_dsl::worker_id.eq(worker_id_arg));
        }

        if let Some(state_arg) = state_cod {
            query = query.filter(tasks_dsl::status.eq_any(state_arg));
        }

        let lock = self.conn.lock().map_err(|e|anyhow!(e.to_string()))?;
        query.load(lock.deref()).anyhow()
    }
}

impl Resource for TaskpoolImpl {
    fn get_resource_info(&self, resource_id: String) -> Result<Bas64Byte> {
        let lock = self.conn.lock().map_err(|e|anyhow!(e.to_string()))?;
        resource_infos_dsl::resource_infos.filter(resource_infos_dsl::id.eq(resource_id))
            .first(lock.deref())
            .map(|val: ResourceInfo|Bas64Byte::new(val.data))
            .anyhow()
    }

    fn store_resource_info(&self, resource: Vec<u8>) -> Result<String> {
        let resource_id =  Uuid::new_v4().to_string();
        let resource_info = ResourceInfo{
            id: resource_id.clone(),
            data: resource,
            create_at:  Utc::now().timestamp(),
        };

        let lock = self.conn.lock().map_err(|e|anyhow!(e.to_string()))?;
        diesel::insert_into(resource_infos_dsl::resource_infos).values(&resource_info).execute(lock.deref())?;
        Ok(resource_id)
    }
}


#[cfg(test)]
mod tests{

    #[test]
    pub fn test_status() {

    }
}
