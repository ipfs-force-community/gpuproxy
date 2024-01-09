use anyhow::{anyhow, Result};
use clap::value_parser;
use std::borrow::{Borrow, BorrowMut};
use std::convert::TryFrom;
use std::fmt::format;
use std::process::exit;
use std::rc::Rc;

use crate::cli::utils::{short_msg, timestamp_to_string};
use crate::proxy_rpc::rpc::{get_proxy_api, GpuServiceRpcClient};
use chrono::{DateTime, Local, LocalResult, NaiveDateTime, TimeZone, Utc};
use clap::{Arg, ArgAction, ArgMatches, Command};
use entity::tasks::Model as Task;
use entity::{TaskState, TaskType};
use tabled::builder::Builder;
use tabled::settings::style::Style;

pub async fn task_cmds<'a>() -> Command<'a> {
    Command::new("task")
        .arg_required_else_help(true)
        .about("task command")
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
        .subcommand(
            Command::new("get")
                .about("get task detail")
                .args(&[Arg::new("id")
                    .last(true)
                    .takes_value(true)
                    .required(true)
                    .help("task id")]),
        )
}

pub async fn tasks_command(task_m: &&ArgMatches) -> Result<()> {
    match task_m.subcommand() {
        Some(("list", ref sub_m)) => list_tasks(sub_m).await,
        Some(("update-state", ref sub_m)) => update_status_by_id(sub_m).await,
        Some(("get", ref sub_m)) => get_task(sub_m).await,
        _ => Err(anyhow!("command not found")),
    }
}

pub async fn get_task(sub_m: &&ArgMatches) -> Result<()> {
    let url: String = sub_m
        .get_one::<String>("url")
        .ok_or_else(|| anyhow!("url flag not found"))?
        .clone();

    let id: String = sub_m
        .get_one::<String>("id")
        .ok_or_else(|| anyhow!("id argument not found"))?
        .clone();

    let server_api = get_proxy_api(url).await?;
    let task = server_api.get_task(id).await?;
    print_one_task(task)
}

pub async fn list_tasks(sub_m: &&ArgMatches) -> Result<()> {
    let url: String = sub_m
        .get_one::<String>("url")
        .ok_or_else(|| anyhow!("url flag not found"))?
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
    let server_api = get_proxy_api(url).await?;
    let tasks = server_api.list_task(None, states).await?;
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

    let server_api = get_proxy_api(url).await?;
    server_api.update_status_by_id(ids, state).await?;
    println!("update state success");
    Ok(())
}

fn print_task(tasks: Vec<Task>) -> Result<()> {
    let mut builder = Builder::new();

    builder.set_header([
        "Id",
        "Miner",
        "Type",
        "State",
        "ResourceId",
        "Comment",
        "Err",
        "CreateAt",
        "StartAt",
        "CompleteAt",
    ]);

    for task in tasks {
        builder.push_record([
            task.id.as_str(),
            task.miner.as_str(),
            task.task_type.to_string().as_str(),
            state_to_string(task.state).as_str(),
            task.resource_id.as_str(),
            short_msg(task.comment, 30).as_str(),
            short_msg(task.error_msg, 20).as_str(),
            timestamp_to_string(task.create_at).as_str(),
            timestamp_to_string(task.start_at).as_str(),
            timestamp_to_string(task.complete_at).as_str(),
        ]);
    }
    println!("{}", builder.build().with(Style::ascii()));
    Ok(())
}

fn print_one_task(task: Task) -> Result<()> {
    let mut table = Builder::new();

        table.set_header(["Name", "Value"])
        .push_record(["Id", task.id.as_str()])
        .push_record(["Miner", task.miner.as_str()])
        .push_record(["Miner", task.miner.as_str()])
        .push_record(["Type", task.task_type.to_string().as_str()])
        .push_record(["State", state_to_string(task.state).as_str()])
        .push_record(["ResourceId", task.resource_id.as_str()])
        .push_record(["Comment", task.comment.as_str()])
        .push_record(["Err", task.error_msg.as_str()])
        .push_record(["CreateAt", timestamp_to_string(task.create_at).as_str()])
        .push_record(["StartAt", timestamp_to_string(task.start_at).as_str()])
        .push_record(["CompleteAt", timestamp_to_string(task.complete_at).as_str()]);

    println!("{}", table.build().with(Style::ascii()));
    Ok(())
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
