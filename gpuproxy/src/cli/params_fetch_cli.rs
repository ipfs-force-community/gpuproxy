use crate::params_fetch;
use clap::{Arg, ArgMatches, Command};

pub async fn fetch_params_cmds<'a>() -> Command<'a> {
    Command::new("paramfetch").about("download params for c2 task").args(&[Arg::new("sector-size")
        .long("sector-size")
        .multiple_occurrences(true)
        .multiple_values(true)
        .takes_value(true)
        .env("C2PROXY_SECTOR_SIZE")
        .help("specify size of params to fetch")])
}
pub async fn fetch_params_command(params_fetch_sub_m: &&ArgMatches) {
    let sizes: Vec<_> = params_fetch_sub_m.values_of_t("sector-size").unwrap();
    params_fetch::download_sector_size(Some(sizes))
}
