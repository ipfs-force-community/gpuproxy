use diesel::prelude::*;
pub mod schema;

use schema::tasks;
use std::sync::{Mutex};

#[derive(Queryable)]
pub struct Task {
    pub id: i64,
    pub miner: String,
    pub prove_id: String,
    pub sector_id: u64,
    pub phase1_output: Vec<u8>,
    pub proof: Vec<u8>,
    pub status: u8,
    pub create_at: i64,
    pub complete_at: i64,
}

#[derive(Insertable, Queryable)]
#[table_name = "tasks"]
pub struct NewTask {
    pub miner: String,
    pub prove_id: String,
    pub sector_id: i64,
    pub phase1_output: Vec<u8>,
    pub proof: Vec<u8>,
    pub status: i64,
    pub create_at: i64,
    pub complete_at: i64,
}

pub fn establish_connection(conn_string: &str) -> Mutex<SqliteConnection> {
    Mutex::new(SqliteConnection::establish(conn_string).unwrap_or_else(|_| panic!("Error connecting to {}", conn_string)))
}
