use std::ops::DerefMut;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use clap::{Arg, Command};
use entity::{TaskState, TaskType};
use fil_types::ActorID;
use filecoin_proofs::ProverId;
use filecoin_proofs_api::seal::SealCommitPhase1Output;
use filecoin_proofs_api::seal::SealCommitPhase2Output;
use gpuproxy::proxy_rpc::rpc::get_proxy_api;
use gpuproxy::proxy_rpc::rpc::GpuServiceRpcClient;
use gpuproxy::proxy_rpc::rpc::WrapClient;
use gpuproxy::utils::Base64Byte;
use gpuproxy::{cli, utils};
use log::*;
use serde::{Deserialize, Serialize};
use serde_json::{from_str, to_string};
use storage_proofs_core::sector::SectorId;
use tokio::{
    io::{self, AsyncBufReadExt, AsyncWrite, AsyncWriteExt, BufReader, BufWriter},
    sync::Mutex,
    time::{sleep, Duration},
};

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
    pub task: T,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Response<T> {
    pub id: u64,
    pub err_msg: Option<String>,
    pub output: Option<T>,
}

fn ready_msg(name: &str) -> String {
    format!("{} processor ready\n", name)
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
                        .value_parser(clap::value_parser!(u64))
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

    if let Err(e) = match app_m.subcommand() {
        Some(("run", ref sub_m)) => {
            cli::set_worker_env(sub_m);

            let url: String = sub_m.get_one::<String>("gpuproxy-url").unwrap().clone();
            let pool_task_interval: u64 = *sub_m.get_one::<u64>("poll-task-interval").unwrap();

            let cfg = C2PluginCfg {
                url,
                pool_task_interval,
            };
            run(cfg).await
        }
        _ => Ok(()),
    } {
        println!("exec cluster plugin error {e}")
    }
}

async fn run(cfg: C2PluginCfg) -> Result<()> {
    let c2_stage_name = "c2";

    let mut stdout = BufWriter::new(io::stdout());

    write_all(&mut stdout, ready_msg(c2_stage_name).as_bytes()).await?;

    let pid = std::process::id();
    let output = Arc::new(Mutex::new(stdout));
    let mut input = BufReader::new(io::stdin()).lines();

    info!("stage {}, pid {} processor ready", c2_stage_name, pid);

    while let Some(line) = input.next_line().await.context("read line from stdin")? {
        let req: Request<C2Input> = match from_str(&line).context("unmarshal request") {
            Ok(r) => r,
            Err(e) => {
                error!("unmarshal request: {:?}", e);
                continue;
            }
        };

        debug!("process request id {}, size {}", req.id, line.len());
        let cfg_clone = cfg.clone();

        let output_cloned = output.clone();
        tokio::spawn(async move {
            if let Err(e) = process_request(cfg_clone, req, output_cloned).await {
                error!("failed: {:?}", e);
            }
        });
    }

    Err(anyhow!("got empty line, parent might be out"))
}

async fn process_request(
    cfg: C2PluginCfg,
    req: Request<C2Input>,
    output: Arc<Mutex<impl AsyncWrite + Unpin>>,
) -> Result<()> {
    let id = req.id;
    let sector_id = req.task.sector_id;
    let input = req.task;

    trace!("input: {:?}", input);

    let params_bytes = serde_json::to_vec(&input).context("unmarshal c2 input")?;
    let miner_addr = forest_address::Address::new_id(input.miner_id);

    let proxy_client = get_proxy_api(cfg.url.clone())
        .await
        .context("connect gpu proxy url")?;

    let task_id = utils::gen_task_id(miner_addr, TaskType::C2, &params_bytes);
    let task_result = proxy_client.get_task(task_id.clone()).await;
    match task_result {
        Ok(task) => {
            //write before check task status, and reset task if task is in Error state
            if task.state == TaskState::Error {
                proxy_client
                    .update_status_by_id(vec![task_id.clone()], TaskState::Init)
                    .await?;
                info!(
                    "reset failed task miner_id {} task_id {}, sector_id {}",
                    input.miner_id,
                    task_id.clone(),
                    sector_id.clone()
                );
            } else {
                info!(
                    "trace exit task miner_id {} task_id {}, sector_id {}",
                    input.miner_id,
                    task_id.clone(),
                    sector_id.clone()
                );
            }
        }
        Err(e) => {
            let err_msg = e.to_string();
            if err_msg.contains("not found") {
                //write new record
                proxy_client
                    .add_task(
                        miner_addr.to_string(),
                        TaskType::C2,
                        Base64Byte(params_bytes),
                    )
                    .await
                    .context("add task")?;
                info!(
                    "create new task miner_id {}  task_id{} sector_id {} successfully",
                    input.miner_id,
                    task_id.clone(),
                    sector_id
                );
            } else {
                return Err(e);
            }
        }
    }

    let resp = match trace_task_result(cfg, sector_id, task_id, proxy_client).await {
        Ok(out) => Response {
            id,
            err_msg: None,
            output: Some(out),
        },

        Err(e) => Response {
            id,
            err_msg: Some(format!("{:?}", e)),
            output: None,
        },
    };

    let mut res_str = to_string(&resp).context("marshal response")?;
    res_str.push('\n');
    let mut output = output.lock().await;
    write_all(output.deref_mut(), res_str.as_bytes())
        .await
        .context("write response to stdout")?;
    debug!("response written");
    Ok(())
}

async fn trace_task_result(
    cfg: C2PluginCfg,
    sector_id: SectorId,
    task_id: String,
    proxy_client: WrapClient,
) -> Result<SealCommitPhase2Output> {
    let duration = Duration::from_secs(cfg.pool_task_interval);
    loop {
        debug!("trace task {task_id} {duration:?}");
        sleep(duration).await;
        match proxy_client.get_task(task_id.clone()).await {
            Ok(task) => {
                if task.state == TaskState::Error {
                    //发生错误 退出当前执行的任务
                    return Err(anyhow!(
                        "task {} sector {} error reason:{}",
                        task.id,
                        sector_id,
                        task.error_msg
                    ));
                } else if task.state == TaskState::Completed {
                    return Ok(SealCommitPhase2Output { proof: task.proof });
                } else {
                    continue;
                }
            }
            Err(e) => {
                error!("error {e} when trace task {task_id} {sector_id}");
                continue;
            }
        }
    }
}

#[inline]
async fn write_all<W: AsyncWrite + Unpin>(w: &mut W, data: &[u8]) -> io::Result<()> {
    w.write_all(data).await?;
    w.flush().await
}
