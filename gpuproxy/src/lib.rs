#![feature(result_flattening)]
#![feature(async_closure)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#[macro_use(defer)]
extern crate scopeguard;

pub mod config;
pub mod proof_rpc;
pub mod resource;
pub mod utils;
pub mod params_fetch;
