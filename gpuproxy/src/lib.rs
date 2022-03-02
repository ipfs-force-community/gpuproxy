#![feature(result_flattening)]
#![feature(async_closure)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#[macro_use(defer)]
extern crate scopeguard;

pub mod cli;
pub mod config;
pub mod http_server;
pub mod params_fetch;
pub mod proof_rpc;
pub mod resource;
pub mod utils;
