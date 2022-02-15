use num_enum::{IntoPrimitive, TryFromPrimitive};
use sea_orm::entity::prelude::*;
use serde_repr::*;
use serde::{Serialize, Deserialize};

#[derive(IntoPrimitive, TryFromPrimitive)]
#[repr(i32)]
#[derive(Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr,  EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
pub enum TaskStatus {
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


#[derive(IntoPrimitive, TryFromPrimitive)]
#[repr(i32)]
#[derive(Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr,  EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
pub enum TaskType {
    #[sea_orm(num_value = 0)]
    C2 = 0,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[sea_orm(table_name = "tasks")]
pub struct Model {
    #[sea_orm(primary_key)]
    #[sea_orm(column_type = "Text")]
    pub id: String,
    pub miner: String,
    pub resource_id: String,
    pub proof: String,
    pub worker_id: String,
    pub task_type: TaskType,
    pub error_msg: String,
    pub status: TaskStatus,
    pub create_at: i64,
    pub start_at: i64,
    pub complete_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}