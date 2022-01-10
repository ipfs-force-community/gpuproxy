use std::error::Error;
use std::ffi::CString;
use std::io;
use std::ops::Deref;
use crate::models::{NewTask, Task};
use crate::models::schema::tasks;
use std::sync::{Mutex};
use diesel::prelude::*;
use jsonrpc_core::to_string;

#[derive(Debug)]
pub enum TaskStatus {
    Undefined = 0,
    Init = 1,
    Running = 2,
    Error = 3,
    Completed = 4,
}

pub trait Taskpool {
    fn add(&self, task: NewTask) -> Result<i64, String>;
    fn fetch(&self, id: i64) -> Result<Task, String>;
    fn get_status(&self, id: i64) -> Result<TaskStatus, String>;
}

pub struct TaskpoolImpl {
    conn: Mutex<SqliteConnection>,
}

impl Taskpool for TaskpoolImpl {
    fn add(&self, task: NewTask) -> Result<i64, String> {
        let lock = self.conn.lock().unwrap();
        let result = diesel::insert_into(tasks::table)
            .values(&task)
            .execute(lock.deref());
        match result {
            Ok(val) => Ok(val as i64),
            Err(e) => Err(e.to_string()),
        }
    }

    fn fetch(&self, id: i64) -> Result<Task, String> {
        todo!()
    }

    fn get_status(&self, id: i64) -> Result<TaskStatus, String> {
        todo!()
    }
}