use crate::proxy_rpc::rpc::{get_proxy_api, GpuServiceRpcClient};
use clap::{Arg, ArgMatches, Command};

pub fn get_worker_arg<'a>() -> Vec<Arg<'a>> {
    [Arg::new("no-gpu")
        .long("no-gpu")
        .env("C2PROXY_NO_GPU")
        .default_value("false")
        .help("disable worker on gpuproxy manager")]
    .to_vec()
}

pub fn set_worker_env(sub_m: &&ArgMatches) {
    if sub_m.value_of_t::<bool>("no-gpu").unwrap_or_else(|e| e.exit()) {
        std::env::set_var("BELLMAN_NO_GPU", "1");
    }
}
