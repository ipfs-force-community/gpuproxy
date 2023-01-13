use std::ops::Deref;
use std::sync::{Arc, Mutex};

use entity::resource_info as ResourceInfos;
use entity::tasks as Tasks;
use entity::worker_info as WorkerInfos;
use entity::workers_state as WorkersStates;
use fil_types::json::vec;
use ResourceInfos::Model as ResourceInfo;
use Tasks::Model as Task;
use WorkerInfos::Model as WorkerInfo;
use WorkersStates::Model as WorkerState;

use crate::utils::Base64Byte;
use crate::utils::*;
use anyhow::{anyhow, Error, Result};
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
pub trait WorkerRepo {
    async fn get_worker_id(&self) -> Result<uuid::Uuid>;
}

/// Persist task related data and can implement specific storage media to save data
#[async_trait]
pub trait ResourceRepo {
    async fn has_resource(&self, resource_id: String) -> Result<bool>;
    async fn get_resource_info(&self, resource_id: String) -> Result<Base64Byte>;
    async fn store_resource_info(&self, resource_id: String, resource: Vec<u8>) -> Result<String>;
}


#[async_trait]
pub trait TaskRepo {
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
    async fn update_status_by_id(&self, tids: Vec<String>, status: TaskState) -> Result<()>;
    async fn list_task(
        &self,
        worker_id_arg: Option<String>,
        state: Option<Vec<entity::TaskState>>,
    ) -> Result<Vec<Task>>;
    async fn fetch_one_todo(
        &self,
        worker_id_arg: String,
        types: Option<Vec<entity::TaskType>>,
    ) -> Result<Task>;
    async fn fetch_uncompleted(&self, worker_id_arg: String) -> Result<Vec<Task>>;
    async fn record_error(&self, worker_id_arg: String, tid: String, err_msg: String)
                          -> Result<()>;
    async fn record_proof(&self, worker_id_arg: String, tid: String, proof: Vec<u8>) -> Result<()>;
}

#[async_trait]
pub trait WorkerStateRepo {
    async fn report_worker_info(
        &self,
        worker_id_arg: String,
        ips: String,
        support_types: String,
    ) -> Result<()>;

    async fn list_worker(&self) -> Result<Vec<WorkerState>>;
    async fn delete_worker_by_worker_id(&self, worker_id_arg: String) -> Result<()>;
    async fn delete_worker_by_id(&self, id: String) -> Result<()>;
    async fn get_worker_by_worker_id(&self, worker_id_arg: String) -> Result<WorkerState>;
    async fn get_worker_by_id(&self, id: String) -> Result<WorkerState>;
    async fn get_offline_worker(&self, dur: i64) -> Result<Vec<WorkerState>>;
}

pub trait Repo: ResourceRepo + WorkerRepo + TaskRepo + WorkerStateRepo{}
impl<T> Repo for T where T:ResourceRepo +  WorkerRepo + TaskRepo + WorkerStateRepo{}

pub struct DbOpsImpl {
    conn: DatabaseConnection,
}

impl DbOpsImpl {
    pub fn new(conn: DatabaseConnection) -> Self {
        DbOpsImpl { conn }
    }
}

#[async_trait]
impl WorkerRepo for DbOpsImpl {
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
impl TaskRepo for DbOpsImpl {
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

    async fn update_status_by_id(&self, tids: Vec<String>, status: TaskState) -> Result<()> {
        Tasks::Entity::update_many()
            .col_expr(Tasks::Column::State, Expr::value(status))
            .filter(Tasks::Column::Id.is_in(tids))
            .exec(&self.conn)
            .await
            .map(|_| ())
            .anyhow()
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
        query = query.order_by(Tasks::Column::CreateAt, Order::Desc);
        query.all(&self.conn).await.anyhow()
    }

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
    ) -> Result<()> {
        Tasks::Entity::update_many()
            .col_expr(Tasks::Column::State, Expr::value(TaskState::Error))
            .col_expr(Tasks::Column::WorkerId, Expr::value(worker_id_arg.clone()))
            .col_expr(Tasks::Column::ErrorMsg, Expr::value(err_msg_str.clone()))
            .filter(Tasks::Column::Id.eq(tid.clone()))
            .exec(&self.conn)
            .await
            .map(|_| {
                info!(
                    "worker {} mark task {} as error reason:{}",
                    worker_id_arg, tid, err_msg_str
                );
            })
            .anyhow()
    }

    async fn record_proof(
        &self,
        worker_id_arg: String,
        tid: String,
        proof_str: Vec<u8>,
    ) -> Result<()> {
        Tasks::Entity::update_many()
            .col_expr(Tasks::Column::State, Expr::value(TaskState::Completed))
            .col_expr(Tasks::Column::WorkerId, Expr::value(worker_id_arg.clone()))
            .col_expr(Tasks::Column::Proof, Expr::value(proof_str))
            .col_expr(Tasks::Column::ErrorMsg, Expr::value(""))
            .col_expr(
                Tasks::Column::CompleteAt,
                Expr::value(Utc::now().timestamp()),
            )
            .filter(Tasks::Column::Id.eq(tid.clone()))
            .exec(&self.conn)
            .await
            .map(|_| {
                info!(
                    "worker {} complete task {} successfully",
                    worker_id_arg, tid
                );
            })
            .anyhow()
    }
}

#[async_trait]
impl WorkerStateRepo for DbOpsImpl {
    async fn report_worker_info(
        &self,
        worker_id_arg: String,
        ips: String,
        support_types: String,
    ) -> Result<()> {
        let workers_state_opt: Option<WorkerState> = WorkersStates::Entity::find()
            .filter(WorkersStates::Column::WorkerId.eq(worker_id_arg.clone()))
            .one(&self.conn)
            .await
            .anyhow()?;
        match workers_state_opt {
            Some(worker_state) => {
                let mut worker_state_model: WorkersStates::ActiveModel = worker_state.into();
                worker_state_model.worker_id = Set(worker_id_arg.clone());
                worker_state_model.ips = Set(ips.clone());
                worker_state_model.support_types = Set(support_types.clone());
                worker_state_model.update_at = Set(Utc::now().timestamp());
                worker_state_model
                    .update(&self.conn)
                    .await
                    .anyhow()
                    .map(|_| ())
            }
            None => {
                let new_worker_state_id = Uuid::new_v4().to_string();
                let new_worker = WorkersStates::ActiveModel {
                    id: Set(new_worker_state_id),
                    worker_id: Set(worker_id_arg.clone()),
                    ips: Set(ips.clone()),
                    support_types: Set(support_types.clone()),
                    update_at: Set(Utc::now().timestamp()),
                    create_at: Set(Utc::now().timestamp()),
                };
                new_worker.insert(&self.conn).await.anyhow().map(|_| ())
            }
        }
    }

    async fn list_worker(&self) -> Result<Vec<WorkerState>> {
        WorkersStates::Entity::find()
            .order_by(WorkersStates::Column::CreateAt, Order::Desc)
            .all(&self.conn)
            .await
            .anyhow()
    }

    async fn delete_worker_by_worker_id(&self, worker_id_arg: String) -> Result<()> {
        WorkersStates::Entity::delete_many()
            .filter(WorkersStates::Column::WorkerId.eq(worker_id_arg))
            .exec(&self.conn)
            .await
            .map(|_| ())
            .anyhow()
    }

    async fn delete_worker_by_id(&self, id: String) -> Result<()> {
        WorkersStates::Entity::delete_many()
            .filter(WorkersStates::Column::Id.eq(id))
            .exec(&self.conn)
            .await
            .map(|_| ())
            .anyhow()
    }

    async fn get_worker_by_worker_id(&self, worker_id_arg: String) -> Result<WorkerState> {
        WorkersStates::Entity::find()
            .filter(WorkersStates::Column::WorkerId.eq(worker_id_arg.clone()))
            .one(&self.conn)
            .await?
            .if_not_found(worker_id_arg + "worker id")
            .anyhow()
    }

    async fn get_worker_by_id(&self, id: String) -> Result<WorkerState> {
        WorkersStates::Entity::find()
            .filter(WorkersStates::Column::Id.eq(id.clone()))
            .one(&self.conn)
            .await?
            .if_not_found(id)
            .anyhow()
    }

    async fn get_offline_worker(&self, dur: i64) -> Result<Vec<WorkerState>> {
        let offline_time = Utc::now().timestamp() - dur;
        WorkersStates::Entity::find()
            .filter(WorkersStates::Column::UpdateAt.lt(offline_time))
            .order_by(WorkersStates::Column::CreateAt, Order::Desc)
            .all(&self.conn)
            .await
            .anyhow()
    }
}

#[async_trait]
impl ResourceRepo for DbOpsImpl {
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
