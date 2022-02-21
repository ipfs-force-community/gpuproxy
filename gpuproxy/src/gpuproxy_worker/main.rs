use crate::db_ops::*;
use crate::worker::Worker;
use clap::{App, AppSettings, Arg};
use gpuproxy::config::*;
use gpuproxy::proof_rpc::*;
use log::*;
use sea_orm::Database;
use simplelog::*;
use std::str::FromStr;
use std::sync::Arc;

use migration::{Migrator, MigratorTrait};

#[tokio::main]
async fn main() {
    let app_m = App::new("gpuproxy-worker")
        .version("0.0.1")
        .setting(AppSettings::ArgRequiredElseHelp)
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .subcommand(
            App::new("run")
                .setting(AppSettings::ArgRequiredElseHelp)
                .about("worker for execute task")
                .args(&[
                    Arg::new("gpuproxy-url")
                        .long("gpuproxy-url")
                        .env("C2PROXY_LISTEN_URL")
                        .default_value("http://127.0.0.1:8888")
                        .help("specify url for connect gpuproxy for get task to excute"),
                    Arg::new("db-dsn")
                        .long("db-dsn")
                        .env("C2PROXY_DSN")
                        .default_value("sqlite://gpuproxy-worker.db")
                        .help("specify sqlite path to store task"),
                    Arg::new("max-c2")
                        .long("max-c2")
                        .env("C2PROXY_MAX_C2")
                        .default_value("1")
                        .help("number of c2 task to run parallelly"),
                    Arg::new("log-level")
                        .long("log-level")
                        .env("C2PROXY_LOG_LEVEL")
                        .default_value("info")
                        .help("set log level for application"),
                ]),
        )
        .get_matches();

    match app_m.subcommand() {
        Some(("run", ref sub_m)) => {
            let url: String = sub_m
                .value_of_t("gpuproxy-url")
                .unwrap_or_else(|e| e.exit());
            let max_c2: usize = sub_m.value_of_t("max-c2").unwrap_or_else(|e| e.exit());
            let db_dsn: String = sub_m.value_of_t("db-dsn").unwrap_or_else(|e| e.exit());
            let log_level: String = sub_m.value_of_t("log-level").unwrap_or_else(|e| e.exit());
            let cfg = ClientConfig::new(
                url,
                db_dsn,
                max_c2,
                "db".to_string(),
                "".to_string(),
                log_level,
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

            let worker_api = Arc::new(proof::get_proxy_api(cfg.url).await.unwrap());
            let worker = worker::LocalWorker::new(
                cfg.max_c2,
                worker_id.to_string(),
                worker_api.clone(),
                worker_api,
            );
            worker.process_tasks().await;
            info!("ready for local worker address worker_id {}", worker_id);
            let () = futures::future::pending().await;
            info!("Shutting Down");
        } // run was used
        _ => {} // Either no subcommand or one not tested for...
    }
}
