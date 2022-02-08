use clap::{App, AppSettings, Arg};
use gpuproxy::proof_rpc::proof::{get_worker_api, GpuServiceRpcClient};


pub fn list_task_cmds<'a>() -> App<'a> {
    App::new("tasks")
        .setting(AppSettings::ArgRequiredElseHelp)
        .about("run daemon for provide service")
        .args(&[
            Arg::new("url")
                .long("url")
                .env("C2PROXY_URL")
                .default_value("127.0.0.1:8888")
                .help("specify url for provide service api service"),
        ])
        .subcommand(
            App::new("list")
                .about("list task status")
                .args(&[
                   // Arg::new("")
                ])
        )
}

pub fn list_tasks(url: String) {
    let worker_api = get_worker_api(url).unwrap();
    let tasks = worker_api.list_task(None, None).unwrap();

    tasks.iter().for_each(|e|{
       // e.task_type
       // println!({}, e.id, e.miner, e.worker_id, e.task_type, e.status, e.error_msg,  e.start_at,e.complete_at,  )
    })
}