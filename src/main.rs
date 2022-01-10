#[macro_use]
extern crate diesel;

mod config;
mod proof_rpc;
mod models;

use std::borrow::BorrowMut;
use crate::config::*;
use crate::proof_rpc::*;

use log::*;
use simplelog::*;
use clap::{App, AppSettings, Arg};

use jsonrpc_http_server::ServerBuilder;
use jsonrpc_http_server::jsonrpc_core::IoHandler;


fn main() {
    TermLogger::init(LevelFilter::Info, Config::default(), TerminalMode::Mixed, ColorChoice::Auto).unwrap();

    let app_m = App::new("c2proxy")
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
                        .default_value("task.db")
                        .help("specify sqlite path to store task")
                ]),
        )
        .get_matches();

    match app_m.subcommand() {
        Some(("run", sub_m)) => {
            let url: String = sub_m.value_of_t("url").unwrap_or_else(|e| e.exit());
            let db_dsn: String = sub_m.value_of_t("db-dsn").unwrap_or_else(|e| e.exit());
            let cfg = ServiceConfig::new(url, db_dsn);

            let mut io = IoHandler::default();

            let db_conn = models::establish_connection(cfg.db_dsn.as_str());
            proof::register(io.borrow_mut(), db_conn);

         //  let path = Path::new(&cfg.url).join("rpc/v0").as_path().to_str().unwrap().to_string();
        //    println!("path {}", path)
            let server = ServerBuilder::new(io)
                .threads(3)
                .start_http(&cfg.url.parse().unwrap())
                .unwrap();

            info!("starting listening {}", cfg.url);
            server.wait();
        } // run was used
        _ => {} // Either no subcommand or one not tested for...
    }
}
