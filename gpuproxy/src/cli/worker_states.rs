use anyhow::{anyhow, Result};
use clap::value_parser;
use std::borrow::{Borrow, BorrowMut};
use std::convert::TryFrom;
use std::fmt::format;
use std::process::exit;
use std::rc::Rc;

use crate::proxy_rpc::rpc::{get_proxy_api, GpuServiceRpcClient};
use chrono::{DateTime, Duration, Local, LocalResult, NaiveDateTime, TimeZone, Utc};
use clap::{Arg, ArgAction, ArgMatches, Command};
use duration_str::parse;
use entity::workers_state::Model as WorkerState;
use tabled::{builder::Builder, Style};

use crate::cli::utils::{short_msg, timestamp_to_string};

pub async fn worker_cmds<'a>() -> Command<'a> {
    Command::new("worker")
        .arg_required_else_help(true)
        .about("worker states command")
        .subcommand(Command::new("list").about("list worker status"))
        .subcommand(
            Command::new("get")
                .about("get worker detail")
                .args(&[Arg::new("id")
                    .last(true)
                    .takes_value(true)
                    .required(true)
                    .help("worker state id or worker id")]),
        )
        .subcommand(
            Command::new("delete")
                .about("delete unused worker")
                .args(&[Arg::new("id")
                    .last(true)
                    .takes_value(true)
                    .required(true)
                    .help("delete worker state id or worker id")]),
        )
        .subcommand(
            Command::new("offline")
                .about("get offline node, ")
                .args(&[Arg::new("dur")
                    .takes_value(true)
                    .default_value("20m")
                    .help("dur means how long the worker not reported status")]),
        )
}

pub async fn worker_command(task_m: &&ArgMatches) -> Result<()> {
    match task_m.subcommand() {
        Some(("list", ref sub_m)) => list_workers(sub_m).await,
        Some(("get", ref sub_m)) => get_worker(sub_m).await,
        Some(("delete", ref sub_m)) => delete_worker(sub_m).await,
        Some(("offline", ref sub_m)) => offline_workers(sub_m).await,
        _ => Err(anyhow!("command not found")),
    }
}

pub async fn list_workers(sub_m: &&ArgMatches) -> Result<()> {
    let url: String = sub_m
        .get_one::<String>("url")
        .ok_or_else(|| anyhow!("url flag not found"))?
        .clone();

    let server_api = get_proxy_api(url).await?;
    let workers = server_api.list_worker().await?;
    print_worker(workers)
}

pub async fn get_worker(sub_m: &&ArgMatches) -> Result<()> {
    let url: String = sub_m
        .get_one::<String>("url")
        .ok_or_else(|| anyhow!("url flag not found"))?
        .clone();

    let id: String = sub_m
        .get_one::<String>("id")
        .ok_or_else(|| anyhow!("id argument not found"))?
        .clone();

    let server_api = get_proxy_api(url).await?;
    let mut workers_result = server_api.get_worker_by_id(id.clone()).await;
    if workers_result.is_err() {
        workers_result = server_api.get_worker_by_worker_id(id.clone()).await;
    }
    if let Err(err) = workers_result {
        return Err(anyhow!("maybe {} is not id or worker id {}", id, err));
    }
    print_one_worker(workers_result?)
}

pub async fn delete_worker(sub_m: &&ArgMatches) -> Result<()> {
    let url: String = sub_m
        .get_one::<String>("url")
        .ok_or_else(|| anyhow!("url flag not found"))?
        .clone();

    let id: String = sub_m
        .get_one::<String>("id")
        .ok_or_else(|| anyhow!("id argument not found"))?
        .clone();

    let server_api = get_proxy_api(url).await?;
    server_api.delete_worker_by_id(id.clone()).await?;
    server_api.delete_worker_by_worker_id(id.clone()).await?;
    Ok(())
}

pub async fn offline_workers(sub_m: &&ArgMatches) -> Result<()> {
    let url: String = sub_m
        .get_one::<String>("url")
        .ok_or_else(|| anyhow!("url flag not found"))?
        .clone();

    let dur_str: String = sub_m
        .get_one::<String>("dur")
        .ok_or_else(|| anyhow!("dur flag not found"))?
        .clone();

    let offline_check_point = parse(dur_str.as_str())?.as_secs() as i64;
    let server_api = get_proxy_api(url).await?;
    let workers = server_api.get_offline_worker(offline_check_point).await?;
    print_worker(workers)
}

fn print_worker(workers: Vec<WorkerState>) -> Result<()> {
    let mut builder = Builder::default().set_header([
        "Id",
        "WorkerId",
        "IPs",
        "SupportTypes",
        "CreateAt",
        "UpdateAt",
    ]);

    for worker in workers {
        builder = builder.add_row([
            worker.id.as_str(),
            worker.worker_id.as_str(),
            short_msg(worker.ips, 20).as_str(),
            worker.support_types.as_str(),
            timestamp_to_string(worker.create_at).as_str(),
            timestamp_to_string(worker.update_at).as_str(),
        ]);
    }

    let table = builder.build().with(Style::ascii());
    println!("{}", table);
    Ok(())
}

fn print_one_worker(worker: WorkerState) -> Result<()> {
    let table = Builder::default()
        .set_header(["Name", "Value"])
        .add_row(["Id", worker.id.as_str()])
        .add_row(["WorkerId", worker.worker_id.as_str()])
        .add_row(["IPs", worker.ips.as_str()])
        .add_row(["SupportTypes", worker.support_types.as_str()])
        .add_row(["CreateAt", timestamp_to_string(worker.create_at).as_str()])
        .add_row(["UpdateAt", timestamp_to_string(worker.update_at).as_str()])
        .build()
        .with(Style::ascii());
    println!("{}", table);
    Ok(())
}
