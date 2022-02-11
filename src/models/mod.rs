pub mod schema;
pub mod migrations;
use std::fmt::{Debug};
use schema::tasks;
use schema::worker_infos;
use schema::resource_infos;
use serde::{Serialize, Deserialize};
use serde::{Serializer, Deserializer};
use std::io::Write;
use diesel::prelude::*;
use serde_repr::*;
use diesel::backend::Backend;
use diesel::deserialize::{self, FromSql};
use diesel::serialize::{self, Output, ToSql};
use diesel::sql_types::*;
use num_enum::{IntoPrimitive, TryFromPrimitive};

#[derive(IntoPrimitive, TryFromPrimitive)]
#[repr(i32)]
#[derive(Serialize_repr, Deserialize_repr)]
#[derive(Debug, Clone, Copy, PartialEq, AsExpression, FromSqlRow)]
#[sql_type = "Integer"]
pub enum TaskStatus {
    Undefined = 0,
    Init = 1,
    Running = 2,
    Error = 3,
    Completed = 4,
}

impl<DB> ToSql<Integer, DB> for TaskStatus
    where
        DB: Backend,
        i32: ToSql<Integer, DB>,
{
    fn to_sql<W: Write>(&self, out: &mut Output<W, DB>) -> serialize::Result {
        (*self as i32).to_sql(out)
    }
}


impl<DB> FromSql<Integer, DB> for TaskStatus
    where
        DB: Backend,
        i32: FromSql<Integer, DB>,
{
    fn from_sql(bytes: Option<&DB::RawValue>) -> deserialize::Result<Self> {
        match i32::from_sql(bytes)? {
            0 => Ok(TaskStatus::Undefined),
            1 => Ok(TaskStatus::Init),
            2 => Ok(TaskStatus::Running),
            3 => Ok(TaskStatus::Error),
            4 => Ok(TaskStatus::Completed),
            x => Err(format!("Unrecognized variant {}", x).into()),
        }
    }
}

#[derive(IntoPrimitive, TryFromPrimitive)]
#[repr(i32)]
#[derive(Serialize_repr, Deserialize_repr)]
#[derive(Debug, Clone, Copy, PartialEq, AsExpression, FromSqlRow)]
#[sql_type = "Integer"]
pub enum TaskType {
    C2 = 0,
}


impl<DB> ToSql<Integer, DB> for TaskType
    where
        DB: Backend,
        i32: ToSql<Integer, DB>,
{
    fn to_sql<W: Write>(&self, out: &mut Output<W, DB>) -> serialize::Result {
        (*self as i32).to_sql(out)
    }
}


impl<DB> FromSql<Integer, DB> for TaskType
    where
        DB: Backend,
        i32: FromSql<Integer, DB>,
{
    fn from_sql(bytes: Option<&DB::RawValue>) -> deserialize::Result<Self> {
        match i32::from_sql(bytes)? {
            0 => Ok(TaskType::C2),
            x => Err(format!("Unrecognized variant {}", x).into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Identifiable,Queryable)]
#[serde(rename_all = "camelCase")]
pub struct Task {
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

#[derive(Debug, Insertable)]
#[table_name = "tasks"]
pub struct NewTask {
    pub id: String,
    pub miner: String,
    pub resource_id: String,
    pub worker_id: String,
    pub task_type: TaskType,
    pub status: TaskStatus,
    pub create_at: i64,
}

#[derive(Debug, Serialize, Deserialize,Identifiable, Queryable)]
pub struct WorkerInfo {
    pub id: String,
}

#[derive(Debug, Insertable)]
#[table_name = "worker_infos"]
pub struct NewWorkerInfo {
    pub id: String,
}


#[derive(Debug, Queryable, Identifiable, Insertable)]
#[table_name = "resource_infos"]
pub struct ResourceInfo {
    pub id: String,
    pub data: Vec<u8>,
    pub create_at: i64,
}


pub fn establish_connection(conn_string: &str) -> SqliteConnection {
    SqliteConnection::establish(conn_string).unwrap_or_else(|_| panic!("Error connecting to {}", conn_string))
}

#[derive(Debug)]
pub struct Base64Byte(Vec<u8>);

impl Base64Byte {
    pub fn new(data: Vec<u8>) -> Self {
        Base64Byte(data)
    }
}

impl Into<Vec<u8>> for Base64Byte {
    fn into(self) -> Vec<u8> {
       self.0
    }
}

impl Serialize for Base64Byte {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        serializer.serialize_str( base64::encode(&self.0).as_str())
    }
}

impl<'de> Deserialize<'de> for Base64Byte {
    fn deserialize<D>(deserializer: D) -> Result<Base64Byte, D::Error>
        where
            D: Deserializer<'de>,
    {
        let bytes_str = <String>::deserialize(deserializer)?;
        Ok(Base64Byte(base64::decode(bytes_str).unwrap()))
    }
}