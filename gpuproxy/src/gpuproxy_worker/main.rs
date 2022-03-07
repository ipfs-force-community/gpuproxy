use crate::db_ops::*;
use crate::worker::Worker;

use clap::{Arg, Command};
use entity::TaskType;
use gpuproxy::cli;
use gpuproxy::config::*;
use gpuproxy::proxy_rpc::*;
use gpuproxy::resource;
use log::*;
use migration::{Migrator, MigratorTrait};
use sea_orm::Database;
use simplelog::*;
use std::str::FromStr;
use std::sync::Arc;
use tokio::signal::ctrl_c;
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
                        .default_value("db")
                        .help("resource type(db, fs)"),
                    Arg::new("fs-resource-path")
                        .long("fs-resource-path")
                        .env("./tar")
                        .default_value("")
                        .help("when resource type is fs, will use this path to read resource"),
                    Arg::new("task-type")
                        .long("task-type")
                        .multiple_values(true)
                        .takes_value(true)
                        .help("task types that worker support (c2 = 0)"),
                ])
                .args(worker_args),
        )
        .get_matches();

    match app_m.subcommand() {
        Some(("run", ref sub_m)) => {
            cli::set_worker_env(sub_m);

            let url: String = sub_m
                .value_of_t("gpuproxy-url")
                .unwrap_or_else(|e| e.exit());
            let max_tasks: usize = sub_m.value_of_t("max-tasks").unwrap_or_else(|e| e.exit());
            let db_dsn: String = sub_m.value_of_t("db-dsn").unwrap_or_else(|e| e.exit());
            let log_level: String = sub_m.value_of_t("log-level").unwrap_or_else(|e| e.exit());
            let resource_type: String = sub_m
                .value_of_t("resource-type")
                .unwrap_or_else(|e| e.exit());
            let fs_resource_type: String = sub_m
                .value_of_t("fs-resource-path")
                .unwrap_or_else(|e| e.exit());
            let task_types = if sub_m.is_present("task-type") {
                let values = sub_m
                    .values_of_t::<i32>("task-type")
                    .unwrap_or_else(|e| e.exit())
                    .into_iter()
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
                task_types,
            );

            let lv = LevelFilter::from_str(cfg.log_level.as_str()).unwrap();
            TermLogger::init(
                lv,
                Config::default(),
                TerminalMode::Mixed,
                ColorChoice::Auto,
            )
            .unwrap();

            let db_conn = Database::connect(cfg.db_dsn.as_str()).await.unwrap();
            Migrator::up(&db_conn, None).await.unwrap();

            let db_ops = db_ops::DbOpsImpl::new(db_conn);
            let worker_id = db_ops.get_worker_id().await.unwrap();

            let worker_api = Arc::new(rpc::get_proxy_api(cfg.url).await.unwrap());
            let resource: Arc<dyn resource::Resource + Send + Sync> = match cfg.resource {
                Resource::Db => worker_api.clone(),
                Resource::FS(path) => Arc::new(resource::FileResource::new(path)),
            };

            let worker = worker::LocalWorker::new(
                cfg.max_tasks,
                worker_id.to_string(),
                cfg.task_types,
                resource,
                worker_api,
            );
            worker.process_tasks().await;
            info!("ready for local worker address worker_id {}", worker_id);
            let mut sig_int = signal(SignalKind::interrupt()).unwrap();
            let mut sig_term = signal(SignalKind::terminate()).unwrap();

            tokio::select! {
                _ = sig_int.recv() => info!("receive SIGINT"),
                _ = sig_term.recv() => info!("receive SIGTERM"),
                _ = ctrl_c() => info!("receive Ctrl C"),
            }
            info!("Shutdown program");
        } // run was used
        _ => {} // Either no subcommand or one not tested for...
    }
}
