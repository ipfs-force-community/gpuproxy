use crate::cli::params_fetch;
use anyhow::{anyhow, Result};
use clap::{value_parser, Arg, ArgAction, ArgMatches, Command};

pub async fn fetch_params_cmds<'a>() -> Command<'a> {
    Command::new("paramfetch")
        .about("download params for c2 task")
        .args(&[Arg::new("sector-size")
            .long("sector-size")
            .action(ArgAction::Append)
            .multiple_values(true)
            .takes_value(true)
            .value_parser(value_parser!(u64))
            .env("C2PROXY_SECTOR_SIZE")
            .help("specify size of params to fetch")])
}

pub async fn fetch_params_command(params_fetch_sub_m: &&ArgMatches) -> Result<()> {
    let sizes: Vec<u64> = params_fetch_sub_m
        .get_many("sector-size")
        .ok_or_else(|| anyhow!("sector-size flag not found"))?
        .copied()
        .collect();
    params_fetch::download_sector_size(Some(sizes));
    Ok(())
}
