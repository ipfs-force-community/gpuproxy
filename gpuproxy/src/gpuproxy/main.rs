use crate::db_ops::*;
use crate::worker::Worker;
use anyhow::{anyhow, Result};
use clap::value_parser;
use clap::{Arg, ArgAction, ArgMatches, Command};
use entity::TaskType;
use gpuproxy::cli;
use gpuproxy::config::*;
use gpuproxy::proxy_rpc::rpc::{ProxyImpl, ONE_GIB};
use gpuproxy::proxy_rpc::*;
use gpuproxy::resource;
use gpuproxy::utils::ensure_db_file;
use jsonrpsee::http_server::{HttpServerBuilder, HttpServerHandle, RpcModule};
use log::*;
use migration::Migrator;
use sea_orm::{ConnectOptions, Database};
use sea_orm_migration::migrator::MigratorTrait;
use simplelog::*;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::signal::ctrl_c;
use tokio::signal::unix::{signal, SignalKind};

#[tokio::main()]
async fn main() {
    let worker_args = cli::get_worker_arg();
    let task_cmds = cli::task_cmds().await;
    let fetch_params_cmds = cli::fetch_params_cmds().await;
    let worker_cmds = cli::worker_cmds().await;
    let app_m = Command::new("gpuproxy")
        .version("0.0.1")
        .args(&[
            Arg::new("url")
                .long("url")
                .env("C2PROXY_URL")
                .global(true)
                .default_value("127.0.0.1:18888")
                .required(false)
                .help("specify url for provide service api service"),
            Arg::new("log-level")
                .long("log-level")
                .global(true)
                .env("C2PROXY_LOG_LEVEL")
                .default_value("info")
                .help("set log level for application"),
        ])
        .arg_required_else_help(true)
        .subcommand(
            Command::new("run")
                .about("run daemon for provide service")
                .args(&[
                    Arg::new("db-dsn")
                        .long("db-dsn")
                        .env("C2PROXY_DSN")
                        .default_value("sqlite://gpuproxy.db")
                        .help("specify sqlite path to store task"),
                    Arg::new("max-tasks")
                        .long("max-tasks")
                        .env("C2PROXY_MAX_TASKS")
                        .value_parser(clap::value_parser!(usize))
                        .default_value("1")
                        .help("number of task to run parallelly"),
                    Arg::new("disable-worker")
                        .long("disable-worker")
                        .env("C2PROXY_DISABLE_WORKER")
                        .required(false)
                        .takes_value(false)
                        .action(ArgAction::SetTrue)
                        .help("disable worker on gpuproxy manager"),
                    Arg::new("resource-type")
                        .long("resource-type")
                        .env("C2PROXY_RESOURCE_TYPE")
                        .default_value("fs")
                        .help("resource type(db(only for test, only for test, have bug for executing too long sql), fs)"),
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
                ])
                .args(worker_args),
        )
        .subcommand(task_cmds)
        .subcommand(fetch_params_cmds)
        .subcommand(worker_cmds)
        .get_matches();

    let exec_result: Result<()> = match app_m.subcommand() {
        Some(("run", ref sub_m)) => start_server(sub_m).await,
        Some(("task", ref sub_m)) => cli::tasks_command(sub_m).await, // task was used
        Some(("worker", ref sub_m)) => cli::worker_command(sub_m).await, // task was used
        Some(("paramfetch", ref sub_m)) => cli::fetch_params_command(sub_m).await, // run was used
        _ => Ok(()), // Either no subcommand or one not tested for...
    };

    if let Err(e) = exec_result {
        println!("{:?}", e);
    }
}

async fn start_server(sub_m: &&ArgMatches) -> Result<()> {
    cli::set_worker_env(sub_m);

    let url = sub_m
        .get_one::<String>("url")
        .ok_or_else(|| anyhow!("url flag not found"))?
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
    let disable_worker = *sub_m
        .get_one::<bool>("disable-worker")
        .ok_or_else(|| anyhow!("disable-worker flag not found"))?;
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

    let cfg = ServiceConfig::new(
        url,
        db_dsn,
        max_tasks,
        disable_worker,
        resource_type,
        fs_resource_type,
        log_level.clone(),
        allow_types,
        debug_sql,
    );

    let lv = LevelFilter::from_str(cfg.log_level.as_str())?;
    TermLogger::init(
        lv,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )?;

    ensure_db_file(&cfg.db_dsn).await?;
    let mut opt = ConnectOptions::new(cfg.db_dsn);
    opt.max_connections(10)
        .min_connections(5)
        .sqlx_logging(cfg.debug_sql)
        .max_lifetime(Duration::from_secs(120))
        .connect_timeout(Duration::from_secs(8))
        .idle_timeout(Duration::from_secs(8));

    let db_conn = Database::connect(opt).await?;
    Migrator::up(&db_conn, None).await?;
    let db_ops = db_ops::DbOpsImpl::new(db_conn);
    let worker_id = db_ops.get_worker_id().await?;
    let arc_pool = Arc::new(db_ops);

    let resource: Arc<dyn resource::ResourceOp + Send + Sync> = match cfg.resource {
        Resource::Db => Arc::new(resource::DbResource::new(arc_pool.clone())),
        Resource::FS(path) => Arc::new(resource::FileResource::new(path)),
    };

    let rpc_module = rpc::register(resource.clone(), arc_pool);
    let (server_addr, handle) = start_api(cfg.url.as_str(), rpc_module).await?;
    info!("starting listening {}", server_addr);

    let worker_api = Arc::new(rpc::get_proxy_api(server_addr.to_string()).await?);
    let worker = worker::LocalWorker::new(
        cfg.max_tasks,
        worker_id.to_string(),
        cfg.allow_types,
        resource,
        worker_api,
    );
    if !cfg.disable_worker {
        worker.register(Some("127.0.0.1".to_owned())).await?;
        worker.process_tasks().await;
        info!("ready for local worker address worker_id {}", worker_id);
    }

    let mut sig_int = signal(SignalKind::interrupt())?;
    let mut sig_term = signal(SignalKind::terminate())?;

    tokio::select! {
        _ = sig_int.recv() => info!("receive SIGINT"),
        _ = sig_term.recv() => info!("receive SIGTERM"),
        _ = ctrl_c() => info!("receive Ctrl C"),
    }
    handle.stop()?;
    info!("Shutdown program");
    Ok(())
} //run cfg

async fn start_api(
    url: &str,
    module: RpcModule<ProxyImpl>,
) -> Result<(SocketAddr, HttpServerHandle)> {
    let server = HttpServerBuilder::default()
        .max_request_body_size(ONE_GIB)
        .build(url.parse::<SocketAddr>()?)?;

    let addr = server.local_addr()?;
    let server_handle = server.start(module)?;

    Ok((addr, server_handle))
}
