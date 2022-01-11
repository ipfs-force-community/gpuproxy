use diesel::prelude::*;
pub mod schema;

use schema::tasks;
use std::sync::{Mutex};

#[derive(Identifiable,Queryable)]
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

#[derive(Insertable, Queryable)]
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
