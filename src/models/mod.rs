pub mod schema;
pub mod migrations;

use diesel::prelude::*;
use schema::tasks;
use schema::worker_infos;
use schema::resource_infos;
use serde::{Serialize, Deserialize};
use serde::{Serializer, Deserializer};

#[derive(Debug, Clone, Serialize, Deserialize, Identifiable,Queryable)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub miner: String,
    pub resource_id: String,
    pub proof: String,
    pub worker_id: String,
    pub task_type: i32,
    pub error_msg: String,
    pub status: i32,
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
    pub task_type: i32,
    pub status: i32,
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
pub struct Bas64Byte(Vec<u8>);

impl Into<Vec<u8>> for Bas64Byte {
    fn into(self) -> Vec<u8> {
       self.0
    }
}

impl Serialize for Bas64Byte {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        println!("asdsadsad");
        serializer.serialize_str( base64::encode(&self.0).as_str())
    }
}

impl<'de> Deserialize<'de> for Bas64Byte {
    fn deserialize<D>(deserializer: D) -> Result<Bas64Byte, D::Error>
        where
            D: Deserializer<'de>,
    {
        let bytes_str = <String>::deserialize(deserializer)?;
        Ok(Bas64Byte(base64::decode(bytes_str).unwrap()))
    }
}