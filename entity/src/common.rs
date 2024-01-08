use num_enum::{IntoPrimitive, TryFromPrimitive};
use sea_orm::entity::prelude::*;
use serde_repr::*;

use std::fmt ;
/// Used to indicate the state of the current task, the inner type is i32
/// 0 Undefined old state
/// 1 Init every new task should be init
/// 2 Running task has fetched by worker by not completed
/// 3 Error have error while running this task
/// 4 Completed task has been calculated
#[repr(i32)]
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    IntoPrimitive,
    TryFromPrimitive,
    Serialize_repr,
    Deserialize_repr,
    EnumIter,
    DeriveActiveEnum,
)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
pub enum TaskState {
    //todo try to implement to_string
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
#[repr(i32)]
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    IntoPrimitive,
    TryFromPrimitive,
    Serialize_repr,
    Deserialize_repr,
    EnumIter,
    DeriveActiveEnum,
)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
pub enum TaskType {
    #[sea_orm(num_value = 0)]
    C2 = 0,
}

//todo add to_string in task type
pub fn task_type_to_string(t: TaskType) -> String {
    match t {
        TaskType::C2 => "C2".to_owned(),
    }
}

impl fmt::Display for TaskType {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", "C2".to_owned())
    }
}