use anyhow::anyhow;
use anyhow::Result;
use clap::value_parser;
use std::borrow::{Borrow, BorrowMut};
use std::convert::TryFrom;
use std::fmt::format;
use std::process::exit;
use std::rc::Rc;

use crate::proxy_rpc::rpc::{get_proxy_api, GpuServiceRpcClient};
use chrono::{DateTime, Local, LocalResult, NaiveDateTime, TimeZone, Utc};
use clap::{Arg, ArgAction, ArgMatches, Command};
use entity::tasks::Model as Task;
use entity::{TaskState, TaskType};
use tabled::{builder::Builder, Style};

pub async fn list_task_cmds<'a>() -> Command<'a> {
    Command::new("tasks")
        .arg_required_else_help(true)
        .about("run daemon for provide service")
        .subcommand(
            Command::new("list")
                .about("list task status")
                .args(&[Arg::new("state")
                    .long("state")
                    .multiple_values(true)
                    .takes_value(true)
                    .value_parser(value_parser!(i32))
                    .help("Init = 1\nRunning = 2\nError = 3\nCompleted = 4")]),
        )
        .subcommand(
            Command::new("update-state")
                .about("update status of task")
                .args(&[
                    Arg::new("state")
                        .long("state")
                        .required(true)
                        .takes_value(true)
                        .value_parser(value_parser!(i32))
                        .help("Init = 1\nRunning = 2\nError = 3\nCompleted = 4"),
                    Arg::new("id")
                        .last(true)
                        .multiple_values(true)
                        .takes_value(true)
                        .required(true)
                        .help("id slice of id"),
                ]),
        )
}

pub async fn tasks_command(task_m: &&ArgMatches) -> Result<()> {
    match task_m.subcommand() {
        Some(("list", ref sub_m)) => list_tasks(sub_m).await, // run was used
        Some(("update-state", ref sub_m)) => update_status_by_id(sub_m).await, // run was used
        _ => Err(anyhow!("command not found")),
    }
}

pub async fn list_tasks(sub_m: &&ArgMatches) -> Result<()> {
    let url: String = sub_m
        .get_one::<String>("url")
        .ok_or_else(|| anyhow!("url falg not found"))?
        .clone();

    let states = if sub_m.contains_id("state") {
        let values = sub_m
            .get_many::<i32>("state")
            .ok_or_else(|| anyhow!("state flag not found"))?
            .copied()
            .into_iter()
            .map(|e| TaskState::try_from(e).unwrap())
            .collect();
        Some(values)
    } else {
        None
    };
    let worker_api = get_proxy_api(url).await?;
    let tasks = worker_api.list_task(None, states).await?;
    print_task(tasks)
}

pub async fn update_status_by_id(sub_m: &&ArgMatches) -> Result<()> {
    let url: String = sub_m
        .get_one::<String>("url")
        .ok_or_else(|| anyhow!("url flag not found"))?
        .to_owned();

    let ids = sub_m
        .get_many::<String>("id")
        .ok_or_else(|| anyhow!("id flag not found"))?
        .cloned()
        .collect();

    let state: TaskState = sub_m
        .get_one::<i32>("state")
        .copied()
        .ok_or_else(|| anyhow!("state flag not found"))?
        .try_into()?;

    let worker_api = get_proxy_api(url).await?;
    if worker_api.update_status_by_id(ids, state).await? {
        println!("update state success");
    }
    Ok(())
}

fn print_task(tasks: Vec<Task>) -> Result<()> {
    let mut builder = Builder::default().set_header([
        "Id",
        "Miner",
        "Type",
        "State",
        "ResourceId",
        "Err",
        "CreateAt",
        "StartAt",
        "CompleteAt",
    ]);

    for task in tasks {
        builder = builder.add_row([
            task.id.as_str(),
            task.miner.as_str(),
            task.task_type.to_string().as_str(),
            state_to_string(task.state).as_str(),
            task.resource_id.as_str(),
            task.error_msg.as_str(),
            unit_time(task.create_at).as_str(),
            unit_time(task.start_at).as_str(),
            unit_time(task.complete_at).as_str(),
        ]);
    }

    let table = builder.build().with(Style::ascii());
    println!("{}", table);
    Ok(())
}

fn unit_time(tm: i64) -> String {
    match Local.timestamp_opt(tm, 0) {
        LocalResult::None => "".to_string(),
        LocalResult::Single(v) => v.to_string(),
        LocalResult::Ambiguous(v1, v2) => format!("{}, {}", v1, v2),
    }
}

fn state_to_string(state: TaskState) -> String {
    match state {
        TaskState::Undefined => "Undefined".to_string(),
        TaskState::Init => "Init".to_string(),
        TaskState::Running => "Running".to_string(),
        TaskState::Error => "Error".to_string(),
        TaskState::Completed => "Completed".to_string(),
    }
}
