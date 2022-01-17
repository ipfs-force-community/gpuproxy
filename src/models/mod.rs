use diesel::prelude::*;
pub mod schema;
use schema::tasks;
use std::sync::{Mutex};
use serde::{Serialize, Deserialize};
use serde::{Serializer, Deserializer};

#[derive(Debug, Serialize, Deserialize, Identifiable,Queryable)]
#[table_name = "tasks"]
pub struct Task {
    pub id: i64,
    pub miner: String,
    pub prove_id: String,
    pub sector_id: i64,
    pub phase1_output: String,
    pub proof: String,
    pub task_type: i32,
    pub error_msg: String,
    pub status: i32,
    pub create_at: i64,
    pub start_at: i64,
    pub complete_at: i64,
}

#[derive(Debug, Insertable, Queryable)]
#[table_name = "tasks"]
pub struct NewTask {
    pub miner: String,
    pub prove_id: String,
    pub sector_id: i64,
    pub phase1_output: String,
    pub task_type: i32,
    pub status: i32,
    pub create_at: i64,
}

pub fn establish_connection(conn_string: &str) -> Mutex<SqliteConnection> {
    Mutex::new(SqliteConnection::establish(conn_string).unwrap_or_else(|_| panic!("Error connecting to {}", conn_string)))
}

#[derive(Debug)]
struct Bas64Byte(Vec<u8>);


impl Serialize for Bas64Byte {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        serializer.serialize_str( base64::encode(&self.0).as_str())
    }
}

impl<'de> Deserialize<'de> for Bas64Byte {
    fn deserialize<D>(deserializer: D) -> Result<Bas64Byte, D::Error>
        where
            D: Deserializer<'de>,
    {
        let bytes_str = <&str>::deserialize(deserializer)?;
        Ok(Bas64Byte(base64::decode(bytes_str).unwrap()))
    }
}