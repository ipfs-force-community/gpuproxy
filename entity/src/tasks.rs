use crate::{TaskState, TaskType};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Task Model, Used to save task-related information, such as task status, type, parameters, and results
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[sea_orm(table_name = "tasks")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, column_type = "Text")]
    pub id: String,
    pub miner: String,
    pub resource_id: String,
    #[sea_orm(column_type = "Binary(BlobSize::Long)")]
    pub proof: Vec<u8>,
    pub worker_id: String,
    pub task_type: TaskType,
    pub error_msg: String,
    pub comment: String,
    pub state: TaskState,
    pub create_at: i64,
    pub start_at: i64,
    pub complete_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
