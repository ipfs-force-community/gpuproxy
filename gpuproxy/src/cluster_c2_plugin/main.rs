#![feature(async_closure)]

use clap::{Arg, Command};
use gpuproxy::cli;
use gpuproxy::proxy_rpc::rpc::GpuServiceRpcClient;
use gpuproxy::proxy_rpc::rpc::WrapClient;
use log::*;

use anyhow::{anyhow, Context, Result};
use filecoin_proofs::ProverId;
use filecoin_proofs_api::seal::SealCommitPhase1Output;
use filecoin_proofs_api::seal::SealCommitPhase2Output;
use serde_json::{from_str, to_string};
use std::io::{stdin, stdout, Write};
use storage_proofs_core::sector::SectorId;
use tokio::time;
use tokio::time::Duration;

use entity::{TaskState, TaskType};
use fil_types::ActorID;
use gpuproxy::proxy_rpc::rpc::get_proxy_api;
use gpuproxy::utils::Base64Byte;
use serde::{Deserialize, Serialize};
use tracing::info_span;

#[derive(Clone, Debug, Serialize, Deserialize)]
struct C2PluginCfg {
    url: String,
    pool_task_interval: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// inputs of stage c2
pub struct C2Input {
    pub c1out: SealCommitPhase1Output,
    pub prover_id: ProverId,
    pub sector_id: SectorId,
    pub miner_id: ActorID,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Request<T> {
    pub id: u64,
    pub data: T,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Response<T> {
    pub id: u64,
    pub err_msg: Option<String>,
    pub result: Option<T>,
}

pub fn ready_msg(name: &str) -> String {
    format!("{} processor ready", name)
}

#[tokio::main]
async fn main() {
    env_logger::init();

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
                    Arg::new("poll-task-interval")
                        .long("poll-task-interval")
                        .env("C2PROXY_POLL_TASK_INTERVAL")
                        .default_value("60")
                        .help("interval for pool task status from gpuproxy server"),
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
            let interval: u64 = sub_m
                .value_of_t("poll-task-interval")
                .unwrap_or_else(|e| e.exit());

            let cfg = C2PluginCfg {
                url,
                pool_task_interval: interval,
            };
            run(cfg).await.unwrap();
        } // run was used
        _ => {} // Either no subcommand or one not tested for...
    }
}

async fn run(cfg: C2PluginCfg) -> Result<()> {
    let c2_stage_name = "c2";

    let mut output = stdout();
    writeln!(output, "{}", ready_msg(c2_stage_name))?;

    let pid = std::process::id();
    let span = info_span!("sub", c2_stage_name, pid);
    let _guard = span.enter();

    let mut line = String::new();
    let input = stdin();

    info!("stage {}, pid {} processor ready", c2_stage_name, pid);
    loop {
        debug!("waiting for new incoming line");
        line.clear();
        let size = input.read_line(&mut line)?;
        if size == 0 {
            return Err(anyhow!("got empty line, parent might be out"));
        }

        let req: Request<C2Input> = match from_str(&line).context("unmarshal request") {
            Ok(r) => r,
            Err(e) => {
                error!("unmarshal request: {:?}", e);
                continue;
            }
        };

        debug!("process request id {}, size {}", req.id, size);
        let cfg_clone = cfg.clone();
        tokio::spawn(
            futures::future::lazy(async move |_| {
                if let Err(e) = process_request(cfg_clone, req).await {
                    error!("failed: {:?}", e);
                }
            })
            .await,
        );
    }
}

async fn process_request(cfg: C2PluginCfg, req: Request<C2Input>) -> Result<()> {
    let id = req.id;
    let input = req.data;
    trace!("input: {:?}", input);

    let params = Base64Byte(serde_json::to_vec(&input).context("unmarshal c2 input")?);
    let miner_addr = forest_address::Address::new_id(input.miner_id).to_string();

    let proxy_client = get_proxy_api(cfg.url.clone())
        .await
        .context("connect gpu proxy url")?;
    let task_id = proxy_client
        .add_task(miner_addr, TaskType::C2, params)
        .await
        .context("add task")?;

    info!(
        "miner_id {} submit task {} successfully",
        input.miner_id, task_id
    );

    let resp = match track_task_result(cfg, task_id, proxy_client).await {
        Ok(out) => Response {
            id: id,
            err_msg: None,
            result: Some(out),
        },

        Err(e) => Response {
            id: id,
            err_msg: Some(format!("{:?}", e)),
            result: None,
        },
    };

    let res_str = to_string(&resp).context("marshal response")?;
    let sout = stdout();
    let mut output = sout.lock();
    writeln!(output, "{}", res_str)?;
    drop(output);

    debug!("response written");
    Ok(())
}

async fn track_task_result(
    cfg: C2PluginCfg,
    task_id: String,
    proxy_client: WrapClient,
) -> Result<SealCommitPhase2Output> {
    let duration = Duration::from_secs(cfg.pool_task_interval);
    loop {
        let mut interval = time::interval(duration);
        interval.tick().await;
        let task = proxy_client.get_task(task_id.clone()).await?;
        if task.state == TaskState::Error {
            //发生错误 退出当前执行的任务
            return Err(anyhow!(
                "got task error while excuting task reason:{}",
                task.error_msg
            ));
        } else if task.state == TaskState::Completed {
            return Ok(SealCommitPhase2Output { proof: task.proof });
        } else {
            continue;
        }
    }
}
