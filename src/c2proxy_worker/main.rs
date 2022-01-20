use c2proxy::config::*;
use c2proxy::proof_rpc::*;
use c2proxy::models::*;
use crate::worker::Worker;
use crate::task_pool::*;
use log::*;
use simplelog::*;
use clap::{App, AppSettings, Arg};
use std::sync::Arc;

fn main() {
    TermLogger::init(LevelFilter::Info, Config::default(), TerminalMode::Mixed, ColorChoice::Auto).unwrap();

    let app_m = App::new("c2proxy-worker")
        .version("0.0.1")
        .setting(AppSettings::ArgRequiredElseHelp)
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .subcommand(
            App::new("run")
                .setting(AppSettings::ArgRequiredElseHelp)
                .about("c2proxy worker for execte compute task")
                .args(&[
                    Arg::new("worker-url")
                        .long("worker-url")
                        .env("C2PROXY_WORKER_URL")
                        .default_value("127.0.0.1:8888")
                        .help("specify url for connect c2proxy for get task to excute"),
                    Arg::new("db-dsn")
                        .long("db-dsn")
                        .env("C2PROXY_DSN")
                        .default_value("task.db")
                        .help("specify sqlite path to store task"),
                ]),
        )
        .get_matches();

    match app_m.subcommand() {
        Some(("run", ref sub_m)) => {
            let url: String = sub_m.value_of_t("url").unwrap_or_else(|e| e.exit());
            let db_dsn: String = sub_m.value_of_t("db-dsn").unwrap_or_else(|e| e.exit());
            let cfg = ClientConfig::new(url, db_dsn);

            let db_conn = establish_connection(cfg.db_dsn.as_str());
            let task_pool = task_pool::TaskpoolImpl::new(db_conn);
            let worker_id = task_pool.get_worker_id().unwrap();
            
            let worker_api =  proof::get_worker_api(cfg.url).unwrap();
            let worker = worker::LocalWorker::new(worker_id.to_string(), Arc::new(worker_api));
            worker.process_tasks();
            info!("ready for local worker address worker_id {}", worker_id);
        } // run was used
        _ => {} // Either no subcommand or one not tested for...
    }
}