#![feature(result_flattening)]

#[macro_use(defer)] 
extern crate scopeguard;

pub mod config;
pub mod proof_rpc;
pub mod resource;
pub mod utils;
