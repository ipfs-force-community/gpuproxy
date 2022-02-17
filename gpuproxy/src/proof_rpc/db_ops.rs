use std::ops::Deref;
use std::sync::{Arc, Mutex};

use entity::tasks as Tasks;
use entity::resource_info as ResourceInfos;
use entity::worker_info as WorkerInfos;
use Tasks::Model as Task;
use ResourceInfos::Model as ResourceInfo;
use WorkerInfos::Model as WorkerInfo;

use anyhow::{anyhow, Result};
use chrono::Utc;
use entity::tasks::{TaskStatus, TaskType};
use log::info;
use uuid::Uuid;
use crate::resource::Resource;
use crate::utils::{*};
use crate::utils::base64bytes::Base64Byte;

use sea_orm::entity::prelude::*;
use sea_orm::prelude::*;
use sea_orm::sea_query::Expr;
use sea_orm::ActiveValue::Set;
use sea_orm::{NotSet, QuerySelect, Unset};
use sea_orm::DatabaseConnection;

use async_trait::async_trait;

#[async_trait]
pub trait WorkerApi {
    async fn get_worker_id(&self) -> Result<uuid::Uuid>;
}

#[async_trait]
pub trait WorkerFetch {
    async fn fetch_one_todo(&self,  worker_id_arg: String) -> Result<Task>;
    async fn fetch_uncomplte(&self, worker_id_arg: String) -> Result<Vec<Task>>;
    async fn record_error(&self,  worker_id_arg: String, tid: String,   err_msg: String) -> Option<anyhow::Error>;
    async fn record_proof(&self,  worker_id_arg: String, tid: String, proof: String) -> Option<anyhow::Error>;
}

#[async_trait]
pub trait Common {
    async fn add_task(&self, miner_arg: forest_address::Address, resource_id: String) -> Result<String>;
    async fn fetch(&self, tid: String) -> Result<Task>;
    async fn fetch_undo(&self) -> Result<Vec<Task>>;
    async fn get_status(&self, tid: String) -> Result<TaskStatus>;
    async fn list_task(&self, worker_id_arg: Option<String>, state: Option<Vec<i32>>) -> Result<Vec<Task>>;
}

pub trait DbOp:WorkerApi+WorkerFetch+Common{}
impl<T> DbOp for T where T: WorkerApi + WorkerFetch + Common {}

pub struct TaskpoolImpl {
    conn: DatabaseConnection,
}

impl TaskpoolImpl {
    pub fn new(conn: DatabaseConnection) -> Self {
        TaskpoolImpl {conn: conn}
    }
}

unsafe impl Send for TaskpoolImpl {}
unsafe impl Sync for TaskpoolImpl {}

#[async_trait]
impl WorkerApi for TaskpoolImpl {
   async fn get_worker_id(&self) -> Result<uuid::Uuid> {
        let worker_info_op: Option<WorkerInfo> = WorkerInfos::Entity::find().one(&self.conn).await?;
        if let Some(worker_info) = worker_info_op {
            let load_worker_id = Uuid::parse_str(worker_info.id.as_str())?;
            info!("load worker id {}", load_worker_id.to_string());
            Ok(load_worker_id)

        } else {
            let uid =  uuid::Uuid::new_v4();
            let new_worker_info = WorkerInfos::ActiveModel{
                id: Set(uid.to_string()),
            };
            let _ = new_worker_info.insert(&self.conn).await?;
            info!("create worker id {}", uid);
            Ok(uid)
        }
    }
}

#[async_trait]
impl WorkerFetch for TaskpoolImpl {
    async fn fetch_one_todo(&self, worker_id_arg: String) -> Result<Task> {
        let undo_task_opt: Option<Task> = Tasks::Entity::find().filter(Tasks::Column::Status.eq(TaskStatus::Init)).one(&self.conn).await?;
        if let Some(undo_task) = undo_task_opt {
            let mut undo_task_active: Tasks::ActiveModel = undo_task.into();
            undo_task_active.status = Set(TaskStatus::Running);
            undo_task_active.worker_id = Set(worker_id_arg);
            undo_task_active.start_at = Set(Utc::now().timestamp());
            undo_task_active.update(&self.conn).await.anyhow()
        }else{
            Err(anyhow!("no task to do for worker"))
        }
    }

    async fn fetch_uncomplte(&self, worker_id_arg: String) -> Result<Vec<Task>>{
            Tasks::Entity::find()
                .filter(Tasks::Column::Status.eq(TaskStatus::Running))
                .filter(Tasks::Column::WorkerId.eq(worker_id_arg))
                .all(&self.conn)
                .await
                .map_err(|e|anyhow!(e.to_string()))
    }

    async fn record_error(&self,  worker_id_arg: String, tid: String, err_msg_str: String) -> Option<anyhow::Error> {
            Tasks::Entity::update_many()
                .col_expr(Tasks::Column::Status, Expr::value(TaskStatus::Error))
                .col_expr(Tasks::Column::WorkerId, Expr::value(worker_id_arg.clone()))
                .col_expr(Tasks::Column::ErrorMsg, Expr::value(err_msg_str.clone()))
                .filter(Tasks::Column::Id.eq(tid.clone()))
                .exec(&self.conn)
                .await
                .map(|e|{
                    info!("worker {} mark task {} as error reason:{}", worker_id_arg, tid, err_msg_str);
                    e
                }).err().map(|e|anyhow!(e.to_string()))
    }

    async fn record_proof(&self,  worker_id_arg: String, tid: String, proof_str: String) -> Option<anyhow::Error> {
            Tasks::Entity::update_many()
                .col_expr(Tasks::Column::Status, Expr::value(TaskStatus::Completed))
                .col_expr(Tasks::Column::WorkerId, Expr::value(worker_id_arg.clone()))
                .col_expr(Tasks::Column::Proof, Expr::value(proof_str))
                .col_expr(Tasks::Column::CreateAt, Expr::value(Utc::now().timestamp()))
                .filter(Tasks::Column::Id.eq(tid.clone()))
                .exec(&self.conn)
                .await
                .map(|e|{
            info!("worker {} complete task {} successfully", worker_id_arg, tid);
            e
        }).err().map(|e|anyhow!(e.to_string()))
    }
}

#[async_trait]
impl Common for TaskpoolImpl {
    async fn add_task(&self, miner_arg: forest_address::Address, resource_id: String) -> Result<String> {
        let miner_noprefix = &miner_arg.to_string()[1..];
        let new_task_id =  Uuid::new_v4().to_string();
        let new_task = Tasks::ActiveModel{
            id: Set(new_task_id.clone()),
            miner: Set(miner_noprefix.to_string()),
            resource_id: Set(resource_id.clone()),
            worker_id: Set("".to_string()),
            task_type: Set(TaskType::C2),
            status:Set(TaskStatus::Init),
            create_at: Set(Utc::now().timestamp()),
            proof: Set("".to_string()),
            error_msg: Set("".to_string()),
            start_at:Set(0),
            complete_at: Set(0),
        };
        println!("fffffff");
        defer! {
            println!("fffffffggg")
        }
        new_task.insert(&self.conn).await.anyhow().and(Ok(new_task_id))
    }

    async fn fetch(&self, tid: String) -> Result<Task> {
        let conn = self.conn.clone();
        futures::executor::block_on(
            Tasks::Entity::find()
                .filter(Tasks::Column::Id.eq(tid.clone()))
                .one(&self.conn)
        )?.if_not_found()
    }

    async fn fetch_undo(&self) -> Result<Vec<Task>> {
            Tasks::Entity::find()
            .filter(Tasks::Column::Status.eq(TaskStatus::Init))
                .all(&self.conn).await.anyhow()
    }

    async fn get_status(&self, tid: String) -> Result<TaskStatus> {
        Tasks::Entity::find()
            .select_only()
            //.column(Tasks::Column::Status)
            .filter(Tasks::Column::Id.eq(tid))
            .one(&self.conn).await?.if_not_found().map(|e|e.status)
    }

    async fn list_task(&self, worker_id_opt: Option<String>, state_cod: Option<Vec<i32>>) -> Result<Vec<Task>> {
        let mut query =  Tasks::Entity::find();
        if let Some(worker_id_arg) = worker_id_opt {
            query = query.filter(Tasks::Column::WorkerId.eq(worker_id_arg));
        }

        if let Some(state_arg) = state_cod {
            query = query.filter(Tasks::Column::Status.is_in(state_arg));
        }
        let conn = self.conn.clone();
       query.all(&self.conn).await.anyhow()
    }
}

#[async_trait]
impl Resource for TaskpoolImpl {
    async fn get_resource_info(&self, resource_id: String) -> Result<Base64Byte> {
            ResourceInfos::Entity::find()
                .filter(ResourceInfos::Column::Id.eq(resource_id))
                .one(&self.conn).await?.if_not_found().map(|val: ResourceInfo| Base64Byte::new(val.data)).anyhow()
    }

    async fn store_resource_info(&self, resource: Vec<u8>) -> Result<String> {
        let resource_id =  Uuid::new_v4().to_string();
        let resource_info = ResourceInfos::ActiveModel{
            id: Set(resource_id.clone()),
            data: Set(resource),
            create_at: Set(Utc::now().timestamp()),
        };

        resource_info.insert(&self.conn).await.map(|_|resource_id).anyhow()
    }
}


#[cfg(test)]
mod tests{

    #[test]
    pub fn test_status() {

    }
}
