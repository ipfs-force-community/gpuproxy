mod cli;
use crate::db_ops::*;
use crate::worker::Worker;
use clap::{App, AppSettings, Arg};
use gpuproxy::config::*;
use gpuproxy::proof_rpc::proof::ProofImpl;
use gpuproxy::proof_rpc::*;
use gpuproxy::resource;
use jsonrpsee::http_server::{HttpServerBuilder, HttpServerHandle, RpcModule};
use log::*;
use sea_orm::Database;
use simplelog::*;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;

use migration::{Migrator, MigratorTrait};

#[tokio::main]
async fn main() {
    let lv = LevelFilter::from_str("trace").unwrap();
    TermLogger::init(
        lv,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .unwrap();

    let list_task_cmds = cli::list_task_cmds().await;
    let app_m = App::new("gpuproxy")
        .version("0.0.1")
        .setting(AppSettings::ArgRequiredElseHelp)
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .subcommand(
            App::new("run")
                .setting(AppSettings::ArgRequiredElseHelp)
                .about("run daemon for provide service")
                .args(&[
                    Arg::new("url")
                        .long("url")
                        .env("C2PROXY_URL")
                        .default_value("127.0.0.1:8888")
                        .help("specify url for provide service api service"),
                    Arg::new("db-dsn")
                        .long("db-dsn")
                        .env("C2PROXY_DSN")
                        .default_value("sqlite://gpuproxy.db")
                        .help("specify sqlite path to store task"),
                    Arg::new("max-c2")
                        .long("max-c2")
                        .env("C2PROXY_MAX_C2")
                        .default_value("1")
                        .help("number of c2 task to run parallelly"),
                    Arg::new("disable-worker")
                        .long("disable-worker")
                        .env("C2PROXY_DISABLE_WORKER")
                        .required(false)
                        .takes_value(false)
                        .default_value("false")
                        .help("disable worker on gpuproxy manager"),
                    Arg::new("log-level")
                        .long("log-level")
                        .env("C2PROXY_LOG_LEVEL")
                        .default_value("info")
                        .help("set log level for application"),
                ]),
        )
        .subcommand(list_task_cmds)
        .get_matches();

    match app_m.subcommand() {
        Some(("run", ref sub_m)) => {
            std::env::set_var("BELLMAN_NO_GPU", "1");
            let url: String = sub_m.value_of_t("url").unwrap_or_else(|e| e.exit());
            let max_c2: usize = sub_m.value_of_t("max-c2").unwrap_or_else(|e| e.exit());
            let db_dsn: String = sub_m.value_of_t("db-dsn").unwrap_or_else(|e| e.exit());
            let log_level: String = sub_m.value_of_t("log-level").unwrap_or_else(|e| e.exit());
            let disable_worker: bool = sub_m
                .value_of_t("disable-worker")
                .unwrap_or_else(|e| e.exit());
            let cfg = ServiceConfig::new(
                url,
                db_dsn,
                max_c2,
                disable_worker,
                "db".to_string(),
                "".to_string(),
                log_level.clone(),
            );

            run_cfg(cfg).await;
        } // run was used
        Some(("tasks", ref sub_m)) => cli::sub_command(sub_m).await, // run was used
        _ => {} // Either no subcommand or one not tested for...
    }
}

async fn run_cfg(cfg: ServiceConfig) {
    let db_conn = Database::connect(cfg.db_dsn.as_str()).await.unwrap();
    Migrator::up(&db_conn, None).await.unwrap();

    let db_ops = db_ops::DbOpsImpl::new(db_conn);
    let worker_id = db_ops.get_worker_id().await.unwrap();
    let arc_pool = Arc::new(db_ops);

    let resource: Arc<dyn resource::Resource + Send + Sync> = if cfg.resource_type == "db" {
        arc_pool.clone()
    } else {
        Arc::new(resource::FileResource::new(cfg.resource_path.clone()))
    };

    let worker = worker::LocalWorker::new(
        cfg.max_c2,
        worker_id.to_string(),
        resource.clone(),
        arc_pool.clone(),
    );

    let rpc_module = proof::register(resource, arc_pool);
    if !cfg.disable_worker {
        worker.process_tasks().await;
        info!("ready for local worker address worker_id {}", worker_id);
    }

    let (server_addr, _handle) = run_server(rpc_module).await.unwrap();
    info!("starting listening {}", server_addr);
    let () = futures::future::pending().await;
    info!("Shutting Down");
} //run cfg

async fn run_server(
    module: RpcModule<ProofImpl>,
) -> anyhow::Result<(SocketAddr, HttpServerHandle)> {
    let server = HttpServerBuilder::default().build("127.0.0.1:8888".parse::<SocketAddr>()?)?;

    let addr = server.local_addr()?;
    let server_handle = server.start(module)?;

    Ok((addr, server_handle))
}
