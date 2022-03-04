use anyhow::anyhow;
use clap::{Arg, Command};
use gpuproxy::cli;
use gpuproxy::proxy_rpc::rpc::GpuServiceRpcClient;
use log::*;

use anyhow::Result;
use filecoin_proofs::ProverId;
use filecoin_proofs_api::seal::SealCommitPhase1Output;
use filecoin_proofs_api::seal::SealCommitPhase2Output;
use serde_json::{from_str, to_string};
use std::io::{stdin, stdout, Write};
use storage_proofs_core::sector::SectorId;
use tokio::time;
use tokio::time::Duration;

use entity::tasks::{TaskState, TaskType};
use fil_types::ActorID;
use gpuproxy::proxy_rpc::rpc::get_proxy_api;
use gpuproxy::utils::Base64Byte;
use serde::{Deserialize, Serialize};
use tracing::info_span;

#[derive(Clone, Debug, Serialize, Deserialize)]
/// inputs of stage c2
pub struct C2Input {
    pub c1out: SealCommitPhase1Output,
    pub prover_id: ProverId,
    pub sector_id: SectorId,
    pub miner_id: ActorID,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Response<T> {
    pub err_msg: Option<String>,
    pub result: Option<T>,
}

pub fn ready_msg(name: &str) -> String {
    format!("{} processor ready", name)
}

#[tokio::main]
async fn main() {
    let worker_args = cli::get_worker_arg();
    let app_m = Command::new("cluster_c2_plugin")
        .version("0.0.1")
        .arg_required_else_help(true)
        .subcommand(
            Command::new("run")
                .about("used to work as c2 plugin in venus-cluster")
                .args(&[
                    Arg::new("gpuproxy-url")
                        .long("gpuproxy-url")
                        .env("C2PROXY_LISTEN_URL")
                        .default_value("http://127.0.0.1:18888")
                        .help("specify url for connect gpuproxy for get task to excute"),
                    Arg::new("log-level")
                        .long("log-level")
                        .env("C2PROXY_LOG_LEVEL")
                        .default_value("info")
                        .help("set log level for application"),
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
            run(url).await.unwrap();
        } // run was used
        _ => {} // Either no subcommand or one not tested for...
    }
}

async fn run(url: String) -> Result<()> {
    let c2_stage_name = "c2";

    let mut output = stdout();
    writeln!(output, "{}", ready_msg(c2_stage_name))?;

    let pid = std::process::id();
    let span = info_span!("sub", c2_stage_name, pid);
    let _guard = span.enter();

    let mut line = String::new();
    let input = stdin();

    info!("processor ready");
    loop {
        debug!("waiting for new incoming line");
        input.read_line(&mut line)?;
        trace!("line: {}", line.as_str());

        debug!("process line");
        let response = match process_line(&url, line.as_str()).await {
            Ok(o) => Response {
                err_msg: None,
                result: Some(o),
            },

            Err(e) => Response {
                err_msg: Some(format!("{:?}", e)),
                result: None,
            },
        };
        trace!("response: {:?}", response);

        debug!("write output");
        let res_str = to_string(&response)?;
        trace!("response: {}", res_str.as_str());
        writeln!(output, "{}", res_str)?;
        line.clear();
    }
}

async fn process_line(url: &str, line: &str) -> Result<SealCommitPhase2Output> {
    let input: C2Input = from_str(line)?;
    trace!("input: {:?}", input);

    let params = Base64Byte(serde_json::to_vec(&input)?);
    let miner_addr = forest_address::Address::new_id(input.miner_id).to_string();

    let proxy_client = get_proxy_api(url.to_string()).await?;
    let task_id = proxy_client
        .add_task(miner_addr, TaskType::C2, params)
        .await?;

    info!(
        "miner_id {} submit task {} successfully",
        input.miner_id, task_id
    );
    loop {
        let mut interval = time::interval(Duration::from_secs(10));
        loop {
            interval.tick().await;
            let task = proxy_client.get_task(task_id.clone()).await?;
            if task.state == TaskState::Error {
                //发生错误 退出当前执行的任务
                return Err(anyhow!(
                    "got task error while excuting task reason:{}",
                    task.error_msg
                ));
            } else if task.state == TaskState::Completed {
                return Ok(SealCommitPhase2Output {
                    proof: base64::decode(task.proof)?,
                });
            } else {
                continue;
            }
        }
    }
}
