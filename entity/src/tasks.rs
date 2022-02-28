use num_enum::{IntoPrimitive, TryFromPrimitive};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use serde_repr::*;

/// Used to indicate the state of the current task, the inner type is i32
/// 0 Undefined old state
/// 1 Init every new task should be init
/// 2 Running task has fetched by worker by not completed
/// 3 Error have error while running this task
/// 4 Completed task has been calculated
#[derive(IntoPrimitive, TryFromPrimitive)]
#[repr(i32)]
#[derive(Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
pub enum TaskState {
    #[sea_orm(num_value = 0)]
    Undefined = 0,
    #[sea_orm(num_value = 1)]
    Init = 1,
    #[sea_orm(num_value = 2)]
    Running = 2,
    #[sea_orm(num_value = 3)]
    Error = 3,
    #[sea_orm(num_value = 4)]
    Completed = 4,
}

/// The type of task, only c2 task supported for now
#[derive(IntoPrimitive, TryFromPrimitive)]
#[repr(i32)]
#[derive(Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
pub enum TaskType {
    #[sea_orm(num_value = 0)]
    C2 = 0,
}

/// Task Model, Used to save task-related information, such as task status, type, parameters, and results
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[sea_orm(table_name = "tasks")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, column_type = "Text")]
    pub id: String,
    pub miner: String,
    pub resource_id: String,
    pub proof: String,
    pub worker_id: String,
    pub task_type: TaskType,
    pub error_msg: String,
    pub state: TaskState,
    pub create_at: i64,
    pub start_at: i64,
    pub complete_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
