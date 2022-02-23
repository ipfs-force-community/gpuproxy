use clap::{Arg, ArgMatches, Command};
use gpuproxy::proof_rpc::proof::{get_proxy_api, GpuServiceRpcClient};

pub async fn list_task_cmds<'a>() -> Command<'a> {
    Command::new("tasks")
        .arg_required_else_help(true)
        .about("run daemon for provide service")
        .args(&[Arg::new("url")
            .long("url")
            .env("C2PROXY_URL")
            .global(true)
            .default_value("http://127.0.0.1:8888")
            .required(false)
            .help("specify url for provide service api service")])
        .subcommand(
                    Command::new("list")
                        .about("list task status")
                        .args(&[
                                // Arg::new("")
                        ]))
}
pub async fn tasks_command(task_m: &&ArgMatches) {
    match task_m.subcommand() {
        Some(("list", ref _sub_m)) => {
            list_tasks(_sub_m).await;
        } // run was used
        _ => {}
    }
}

pub async fn list_tasks(task_m: &&ArgMatches) {
    let url: String = task_m.value_of_t("url").unwrap_or_else(|e| e.exit());
    let worker_api = get_proxy_api(url).await.unwrap();
    let tasks = worker_api.list_task(None, None).await.unwrap();

   /* tasks.iter().for_each(|e|{

    });*/
    println!("{}", serde_json::to_string_pretty(&tasks).unwrap());
}


/*pub async fn update_status_by_id(task_m: &&ArgMatches) {
    let url: String = task_m.value_of_t("url").unwrap_or_else(|e| e.exit());

    let worker_api = get_proxy_api(url).await.unwrap();
    if worker_api.update_status_by_id(ids, state).await.unwrap(){
        println!("update state success");
    }
}*/
