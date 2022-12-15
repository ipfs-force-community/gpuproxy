use std::ops::Deref;
use std::sync::{Arc, Mutex};

use entity::resource_info as ResourceInfos;
use entity::tasks as Tasks;
use entity::worker_info as WorkerInfos;
use fil_types::json::vec;
use ResourceInfos::Model as ResourceInfo;
use Tasks::Model as Task;
use WorkerInfos::Model as WorkerInfo;

use crate::resource::Resource;
use crate::utils::Base64Byte;
use crate::utils::*;
use anyhow::{anyhow, Result};
use chrono::Utc;
use entity::{TaskState, TaskType};
use log::info;
use uuid::Uuid;

use async_trait::async_trait;
use sea_orm::entity::prelude::*;
use sea_orm::prelude::*;
use sea_orm::sea_query::{Expr, Order};
use sea_orm::ActiveValue::Set;
use sea_orm::TransactionTrait;
use sea_orm::{DatabaseConnection, NotSet, QueryOrder, QuerySelect};

#[async_trait]
pub trait WorkerApi {
    async fn get_worker_id(&self) -> Result<uuid::Uuid>;
}

#[async_trait]
pub trait WorkerFetch {
    async fn fetch_one_todo(
        &self,
        worker_id_arg: String,
        types: Option<Vec<entity::TaskType>>,
    ) -> Result<Task>;
    async fn fetch_uncompleted(&self, worker_id_arg: String) -> Result<Vec<Task>>;
    async fn record_error(
        &self,
        worker_id_arg: String,
        tid: String,
        err_msg: String,
    ) -> Option<anyhow::Error>;
    async fn record_proof(
        &self,
        worker_id_arg: String,
        tid: String,
        proof: Vec<u8>,
    ) -> Option<anyhow::Error>;
}

#[async_trait]
pub trait Common {
    async fn add_task(
        &self,
        task_id: String,
        miner_arg: forest_address::Address,
        task_type: TaskType,
        resource_id: String,
    ) -> Result<String>;
    async fn has_task(&self, task_id: String) -> Result<bool>;
    async fn fetch(&self, tid: String) -> Result<Task>;
    async fn fetch_undo(&self) -> Result<Vec<Task>>;
    async fn get_status(&self, tid: String) -> Result<TaskState>;
    async fn update_status_by_id(
        &self,
        tids: Vec<String>,
        status: TaskState,
    ) -> Option<anyhow::Error>;
    async fn list_task(
        &self,
        worker_id_arg: Option<String>,
        state: Option<Vec<entity::TaskState>>,
    ) -> Result<Vec<Task>>;
}

pub trait DbOp: WorkerApi + WorkerFetch + Common {}
impl<T> DbOp for T where T: WorkerApi + WorkerFetch + Common {}

pub struct DbOpsImpl {
    conn: DatabaseConnection,
}

impl DbOpsImpl {
    pub fn new(conn: DatabaseConnection) -> Self {
        DbOpsImpl { conn }
    }
}

#[async_trait]
impl WorkerApi for DbOpsImpl {
    async fn get_worker_id(&self) -> Result<uuid::Uuid> {
        let worker_info_op: Option<WorkerInfo> =
            WorkerInfos::Entity::find().one(&self.conn).await?;
        if let Some(worker_info) = worker_info_op {
            let load_worker_id = Uuid::parse_str(worker_info.id.as_str())?;
            info!("load worker id {}", load_worker_id.to_string());
            Ok(load_worker_id)
        } else {
            let uid = uuid::Uuid::new_v4();
            let new_worker_info = WorkerInfos::ActiveModel {
                id: Set(uid.to_string()),
            };
            let _ = new_worker_info.insert(&self.conn).await?;
            info!("create worker id {}", uid);
            Ok(uid)
        }
    }
}

#[async_trait]
impl WorkerFetch for DbOpsImpl {
    async fn fetch_one_todo(
        &self,
        worker_id_arg: String,
        types: Option<Vec<entity::TaskType>>,
    ) -> Result<Task> {
        self.conn
            .transaction::<_, Task, DbErr>(|txn| {
                Box::pin(async move {
                    let mut query =
                        Tasks::Entity::find().filter(Tasks::Column::State.eq(TaskState::Init));
                    if let Some(state_arg) = types {
                        query = query.filter(Tasks::Column::TaskType.is_in(state_arg));
                    }
                    query = query.order_by(Tasks::Column::CreateAt, Order::Asc);

                    let undo_task_opt: Option<Task> = query.one(txn).await?;
                    if let Some(undo_task) = undo_task_opt {
                        let mut undo_task_active: Tasks::ActiveModel = undo_task.into();
                        undo_task_active.state = Set(TaskState::Running);
                        undo_task_active.worker_id = Set(worker_id_arg);
                        undo_task_active.start_at = Set(Utc::now().timestamp());
                        undo_task_active.update(txn).await
                    } else {
                        Err(DbErr::RecordNotFound("no task to do for worker".to_owned()))
                    }
                })
            })
            .await
            .anyhow()
    }

    async fn fetch_uncompleted(&self, worker_id_arg: String) -> Result<Vec<Task>> {
        Tasks::Entity::find()
            .filter(Tasks::Column::State.eq(TaskState::Running))
            .filter(Tasks::Column::WorkerId.eq(worker_id_arg))
            .order_by(Tasks::Column::CreateAt, Order::Asc)
            .all(&self.conn)
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    async fn record_error(
        &self,
        worker_id_arg: String,
        tid: String,
        err_msg_str: String,
    ) -> Option<anyhow::Error> {
        Tasks::Entity::update_many()
            .col_expr(Tasks::Column::State, Expr::value(TaskState::Error))
            .col_expr(Tasks::Column::WorkerId, Expr::value(worker_id_arg.clone()))
            .col_expr(Tasks::Column::ErrorMsg, Expr::value(err_msg_str.clone()))
            .filter(Tasks::Column::Id.eq(tid.clone()))
            .exec(&self.conn)
            .await
            .map(|e| {
                info!(
                    "worker {} mark task {} as error reason:{}",
                    worker_id_arg, tid, err_msg_str
                );
                e
            })
            .err()
            .map(|e| anyhow!(e.to_string()))
    }

    async fn record_proof(
        &self,
        worker_id_arg: String,
        tid: String,
        proof_str: Vec<u8>,
    ) -> Option<anyhow::Error> {
        Tasks::Entity::update_many()
            .col_expr(Tasks::Column::State, Expr::value(TaskState::Completed))
            .col_expr(Tasks::Column::WorkerId, Expr::value(worker_id_arg.clone()))
            .col_expr(Tasks::Column::Proof, Expr::value(proof_str))
            .col_expr(
                Tasks::Column::CompleteAt,
                Expr::value(Utc::now().timestamp()),
            )
            .filter(Tasks::Column::Id.eq(tid.clone()))
            .exec(&self.conn)
            .await
            .map(|e| {
                info!(
                    "worker {} complete task {} successfully",
                    worker_id_arg, tid
                );
                e
            })
            .err()
            .map(|e| anyhow!(e.to_string()))
    }
}

#[async_trait]
impl Common for DbOpsImpl {
    async fn add_task(
        &self,
        task_id: String,
        miner_arg: forest_address::Address,
        task_type: TaskType,
        resource_id: String,
    ) -> Result<String> {
        let miner_noprefix = &miner_arg.to_string()[1..];
        let new_task = Tasks::ActiveModel {
            id: Set(task_id.clone()),
            miner: Set(miner_noprefix.to_string()),
            resource_id: Set(resource_id.clone()),
            worker_id: Set("".to_string()),
            task_type: Set(task_type),
            state: Set(TaskState::Init),
            create_at: Set(Utc::now().timestamp()),
            proof: Set(vec![]),
            error_msg: Set("".to_string()),
            start_at: Set(0),
            complete_at: Set(0),
        };

        new_task.insert(&self.conn).await.anyhow().and(Ok(task_id))
    }

    async fn has_task(&self, task_id: String) -> Result<bool> {
        Tasks::Entity::find()
            .filter(Tasks::Column::Id.eq(task_id.clone()))
            .count(&self.conn)
            .await
            .map(|count| count > 0)
            .anyhow()
    }

    async fn fetch(&self, tid: String) -> Result<Task> {
        Tasks::Entity::find()
            .filter(Tasks::Column::Id.eq(tid.clone()))
            .order_by(Tasks::Column::CreateAt, Order::Asc)
            .one(&self.conn)
            .await?
            .if_not_found(tid)
    }

    async fn fetch_undo(&self) -> Result<Vec<Task>> {
        Tasks::Entity::find()
            .filter(Tasks::Column::State.eq(TaskState::Init))
            .order_by(Tasks::Column::CreateAt, Order::Asc)
            .all(&self.conn)
            .await
            .anyhow()
    }

    async fn get_status(&self, tid: String) -> Result<TaskState> {
        Tasks::Entity::find()
            .select_only()
            //.column(Tasks::Column::Status)
            .filter(Tasks::Column::Id.eq(tid.clone()))
            .one(&self.conn)
            .await?
            .if_not_found(tid)
            .map(|e| e.state)
    }

    async fn update_status_by_id(
        &self,
        tids: Vec<String>,
        status: TaskState,
    ) -> Option<anyhow::Error> {
        Tasks::Entity::update_many()
            .col_expr(Tasks::Column::State, Expr::value(status))
            .filter(Tasks::Column::Id.is_in(tids))
            .exec(&self.conn)
            .await
            .err()
            .map(|e| anyhow!(e.to_string()))
    }

    async fn list_task(
        &self,
        worker_id_opt: Option<String>,
        state_cod: Option<Vec<entity::TaskState>>,
    ) -> Result<Vec<Task>> {
        let mut query = Tasks::Entity::find();
        if let Some(worker_id_arg) = worker_id_opt {
            query = query.filter(Tasks::Column::WorkerId.eq(worker_id_arg));
        }

        if let Some(state_arg) = state_cod {
            query = query.filter(Tasks::Column::State.is_in(state_arg));
        }
        query = query.order_by(Tasks::Column::CreateAt, Order::Asc);
        query.all(&self.conn).await.anyhow()
    }
}

#[async_trait]
impl Resource for DbOpsImpl {
    async fn has_resource(&self, resource_id: String) -> Result<bool> {
        ResourceInfos::Entity::find()
            .filter(ResourceInfos::Column::Id.eq(resource_id))
            .count(&self.conn)
            .await
            .map(|count| count > 0)
            .anyhow()
    }

    async fn get_resource_info(&self, resource_id: String) -> Result<Base64Byte> {
        ResourceInfos::Entity::find()
            .filter(ResourceInfos::Column::Id.eq(resource_id.clone()))
            .one(&self.conn)
            .await?
            .if_not_found(resource_id)
            .map(|val: ResourceInfo| Base64Byte::new(val.data))
            .anyhow()
    }

    async fn store_resource_info(&self, resource_id: String, resource: Vec<u8>) -> Result<String> {
        let resource_info = ResourceInfos::ActiveModel {
            id: Set(resource_id.clone()),
            data: Set(resource),
            create_at: Set(Utc::now().timestamp()),
        };

        resource_info
            .insert(&self.conn)
            .await
            .map(|_| resource_id)
            .anyhow()
    }
}

#[cfg(test)]
mod tests {

    #[test]
    pub fn test_status() {}
}
