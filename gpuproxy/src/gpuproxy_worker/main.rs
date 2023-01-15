use crate::db_ops::*;
use crate::worker::Worker;

use anyhow::{anyhow, Result};
use clap::value_parser;
use clap::{Arg, ArgAction, ArgMatches, Command};
use entity::TaskType;
use gpuproxy::cli;
use gpuproxy::config::*;
use gpuproxy::proxy_rpc::*;
use gpuproxy::resource;
use gpuproxy::resource::RpcResource;
use gpuproxy::utils::ensure_db_file;
use log::*;
use migration::Migrator;
use sea_orm::{ConnectOptions, Database};
use sea_orm_migration::migrator::MigratorTrait;
use simplelog::*;
use std::str::FromStr;
use std::sync::Arc;
use tokio::signal::unix::{signal, SignalKind};

#[tokio::main]
async fn main() {
    let worker_args = cli::get_worker_arg();
    let app_m = Command::new("gpuproxy-worker")
        .version("0.0.1")
        .arg_required_else_help(true)
        .subcommand(
            Command::new("run")
                .about("worker for execute task")
                .args(&[
                    Arg::new("gpuproxy-url")
                        .long("gpuproxy-url")
                        .env("C2PROXY_LISTEN_URL")
                        .default_value("http://127.0.0.1:18888")
                        .help("specify url for connect gpuproxy for get task to excute"),
                    Arg::new("db-dsn")
                        .long("db-dsn")
                        .env("C2PROXY_DSN")
                        .default_value("sqlite://gpuproxy-worker.db")
                        .help("specify sqlite path to store task"),
                    Arg::new("max-tasks")
                        .long("max-tasks")
                        .env("C2PROXY_MAX_TASKS")
                        .value_parser(clap::value_parser!(usize))
                        .default_value("1")
                        .help("number of c2 task to run parallelly"),
                    Arg::new("log-level")
                        .long("log-level")
                        .env("C2PROXY_LOG_LEVEL")
                        .default_value("info")
                        .help("set log level for application"),
                    Arg::new("resource-type")
                        .long("resource-type")
                        .env("C2PROXY_RESOURCE_TYPE")
                        .default_value("fs")
                        .help(
                            "resource type(db(only for test, have bug for executing too long sql), fs)",
                        ),
                    Arg::new("fs-resource-path")
                        .long("fs-resource-path")
                        .env("C2PROXY_FS_RESOURCE_PATH")
                        .default_value("")
                        .help("when resource type is fs, will use this path to read resource"),
                    Arg::new("allow-type")
                        .long("allow-type")
                        .multiple_values(true)
                        .takes_value(true)
                        .value_parser(value_parser!(i32))
                        .help("task types that worker support (c2 = 0)"),
                    Arg::new("debug-sql")
                        .long("debug-sql")
                        .env("C2PROXY_DEBUG_SQL")
                        .required(false)
                        .action(ArgAction::SetTrue)
                        .help("print sql to debug"),
                    Arg::new("manual-ip")
                        .long("manual-ip")
                        .env("C2PROXY_MANUAL_IP")
                        .required(false)
                        .takes_value(true)
                        .help("set ip manually"),
                ])
                .args(worker_args),
        )
        .get_matches();

    if let Err(e) = match app_m.subcommand() {
        Some(("run", ref sub_m)) => run_worker(sub_m).await,
        _ => Ok(()),
    } {
        println!("exec worker error {e}");
    }
}

async fn run_worker(sub_m: &&ArgMatches) -> Result<()> {
    cli::set_worker_env(sub_m);

    let url = sub_m
        .get_one::<String>("gpuproxy-url")
        .ok_or_else(|| anyhow!("gpuproxy url flag not found"))?
        .clone();
    let max_tasks = *sub_m
        .get_one::<usize>("max-tasks")
        .ok_or_else(|| anyhow!("max-tasks flag not found"))?;
    let db_dsn = sub_m
        .get_one::<String>("db-dsn")
        .ok_or_else(|| anyhow!("db-dsn flag not found"))?
        .clone();
    let log_level = sub_m
        .get_one::<String>("log-level")
        .ok_or_else(|| anyhow!("log-level flag not found"))?
        .clone();
    let debug_sql = *sub_m
        .get_one::<bool>("debug-sql")
        .ok_or_else(|| anyhow!("debug-sql flag not found"))?;
    let resource_type = sub_m
        .get_one::<String>("resource-type")
        .ok_or_else(|| anyhow!("resource-type flag not found"))?
        .clone();
    let fs_resource_type = sub_m
        .get_one::<String>("fs-resource-path")
        .ok_or_else(|| anyhow!("fs-resource-path flag not found"))?
        .clone();
    let manual_ip = sub_m.get_one::<String>("manual-ip");
    let allow_types = if sub_m.contains_id("allow-type") {
        let values = sub_m
            .get_many::<i32>("allow-type")
            .unwrap()
            .copied()
            .map(|e| TaskType::try_from(e).unwrap())
            .collect();
        Some(values)
    } else {
        None
    };

    let cfg = WorkerConfig::new(
        url,
        db_dsn,
        max_tasks,
        resource_type,
        fs_resource_type,
        log_level,
        allow_types,
        debug_sql,
        manual_ip.cloned(),
    );

    let lv = LevelFilter::from_str(cfg.log_level.as_str())?;

    TermLogger::init(
        lv,
        ConfigBuilder::new().set_time_format_rfc3339().build(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )?;

    ensure_db_file(&cfg.db_dsn).await?;
    let mut opt = ConnectOptions::new(cfg.db_dsn);
    opt.max_connections(10)
        .min_connections(5)
        .sqlx_logging(cfg.debug_sql);

    let db_conn = Database::connect(opt).await?;
    Migrator::up(&db_conn, None).await?;
    let db_ops = db_ops::DbOpsImpl::new(db_conn);
    let worker_id = db_ops.get_worker_id().await?;

    let worker_api = Arc::new(rpc::get_proxy_api(cfg.url).await?);
    let resource: Arc<dyn resource::ResourceOp + Send + Sync> = match cfg.resource {
        Resource::Db => Arc::new(RpcResource::new(worker_api.clone())),
        Resource::FS(path) => Arc::new(resource::FileResource::new(path)),
    };

    let worker = worker::LocalWorker::new(
        cfg.max_tasks,
        worker_id.to_string(),
        cfg.allow_types,
        resource,
        worker_api,
    );
    worker.register(cfg.manual_ip.clone()).await?;
    worker.process_tasks().await;
    info!("ready for local worker address worker_id {}", worker_id);
    let mut sig_int = signal(SignalKind::interrupt())?;
    let mut sig_term = signal(SignalKind::terminate())?;

    tokio::select! {
        _ = sig_int.recv() => info!("receive SIGINT"),
        _ = sig_term.recv() => info!("receive SIGTERM"),
    }
    info!("Shutdown program");
    Ok(())
}
