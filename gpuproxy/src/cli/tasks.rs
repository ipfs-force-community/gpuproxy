use std::borrow::{Borrow, BorrowMut};
use std::convert::TryFrom;
use std::rc::Rc;

use crate::proxy_rpc::rpc::{get_proxy_api, GpuServiceRpcClient};
use chrono::{DateTime, Local, NaiveDateTime, TimeZone, Utc};
use clap::{Arg, ArgMatches, Command};
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
                    .help("Init = 1\nRunning = 2\nError = 3\nCompleted = 4")]),
        )
        .subcommand(
            Command::new("update-state")
                .about("update status of task")
                .args(&[
                    Arg::new("id")
                        .long("id")
                        .multiple_values(true)
                        .takes_value(true)
                        .required(true)
                        .help("id slice of id"),
                    Arg::new("state")
                        .long("state")
                        .required(true)
                        .takes_value(true)
                        .help("Init = 1\nRunning = 2\nError = 3\nCompleted = 4"),
                ]),
        )
}
pub async fn tasks_command(task_m: &&ArgMatches) {
    match task_m.subcommand() {
        Some(("list", ref sub_m)) => {
            list_tasks(sub_m).await;
        } // run was used
        Some(("update-state", ref sub_m)) => {
            update_status_by_id(sub_m).await;
        } // run was used
        _ => {}
    }
}

pub async fn list_tasks(sub_m: &&ArgMatches) {
    let url: String = sub_m.value_of_t("url").unwrap_or_else(|e| e.exit());

    let states = if sub_m.is_present("state") {
        let values = sub_m
            .values_of_t::<i32>("state")
            .unwrap_or_else(|e| e.exit())
            .into_iter()
            .map(|e| TaskState::try_from(e).unwrap())
            .collect();
        Some(values)
    } else {
        None
    };
    let worker_api = get_proxy_api(url).await.unwrap();
    let tasks = worker_api.list_task(None, states).await.unwrap();
    print_task(tasks);
}

pub async fn update_status_by_id(sub_m: &&ArgMatches) {
    let url: String = sub_m.value_of_t("url").unwrap_or_else(|e| e.exit());

    let ids = sub_m
        .values_of_t::<String>("id")
        .unwrap_or_else(|e| e.exit())
        .into_iter()
        .collect();

    let state = sub_m
        .value_of_t::<i32>("state")
        .map(|e| TaskState::try_from(e).unwrap())
        .unwrap_or_else(|e| e.exit());

    let worker_api = get_proxy_api(url).await.unwrap();
    if worker_api.update_status_by_id(ids, state).await.unwrap() {
        println!("update state success");
    }
}

fn print_task(tasks: Vec<Task>) {
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
            &task.task_type.to_string(),
            &task.state.to_string(),
            task.resource_id.as_str(),
            task.error_msg.as_str(),
            print_timestamp(task.create_at).as_str(),
            print_timestamp(task.start_at).as_str(),
            print_timestamp(task.complete_at).as_str(),
        ]);
    }

    let table = builder.build().with(Style::ascii());
    println!("{}", table);
}

fn print_timestamp(tm: i64) -> String {
    if tm <= 0 {
        return "".to_string();
    }
    Local.timestamp(tm, 0).to_string()
}
