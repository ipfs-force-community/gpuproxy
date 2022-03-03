#![feature(result_flattening)]
#![feature(async_closure)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#[macro_use(defer)]
extern crate scopeguard;

pub mod cli;
pub mod http_server;
pub mod params_fetch;

pub mod config;
pub mod proxy_rpc;
pub mod resource;
pub mod utils;
