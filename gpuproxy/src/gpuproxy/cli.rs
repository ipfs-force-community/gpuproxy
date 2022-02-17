use clap::{App, AppSettings, Arg, ArgMatches};
use gpuproxy::proof_rpc::proof::{get_proxy_api, GpuServiceRpcClient};


pub async fn list_task_cmds<'a>() -> App<'a> {
    App::new("tasks")
        .setting(AppSettings::ArgRequiredElseHelp)
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .about("run daemon for provide service")
        .args(&[
            Arg::new("url")
                .long("url")
                .env("C2PROXY_URL")
                .default_value("http://127.0.0.1:8888")
                .required(false)
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
pub async fn sub_command(task_m: &&ArgMatches) {
    match task_m.subcommand() {
        Some(("list", ref _sub_m)) => {
            let url: String = task_m.value_of_t("url").unwrap_or_else(|e| e.exit());
            list_tasks(url).await;
        } // run was used
        _ => {}
    }
}
pub async fn list_tasks(url: String) {
    let worker_api = get_proxy_api(url).await.unwrap();
    let tasks = worker_api.list_task(None, None).await.unwrap();

   // tasks.iter().for_each(|e|{
       // e.task_type
       // println!({}, e.id, e.miner, e.worker_id, e.task_type, e.status, e.error_msg,  e.start_at,e.complete_at,  )
   // })
    println!("{}", serde_json::to_string_pretty(&tasks).unwrap());
}