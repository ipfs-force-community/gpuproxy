use crate::proxy_rpc::rpc::{get_proxy_api, GpuServiceRpcClient};
use clap::{Arg, ArgAction, ArgMatches, Command};

pub fn get_worker_arg<'a>() -> Vec<Arg<'a>> {
    [Arg::new("no-gpu")
        .long("no-gpu")
        .env("C2PROXY_NO_GPU")
        .action(ArgAction::SetTrue)
        .help("disable worker on gpuproxy manager")]
    .to_vec()
}

pub fn set_worker_env(sub_m: &&ArgMatches) {
    if *sub_m.get_one::<bool>("no-gpu").unwrap() {
        std::env::set_var("BELLMAN_NO_GPU", "1");
    }
}
