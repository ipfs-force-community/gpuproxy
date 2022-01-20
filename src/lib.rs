#[macro_use]
extern crate diesel;

#[macro_use(defer)] 
extern crate scopeguard;

pub mod config;
pub mod proof_rpc;
pub mod models;