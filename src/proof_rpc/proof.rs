use jsonrpc_core::Result;
use jsonrpc_derive::rpc;
use jsonrpc_http_server::jsonrpc_core::{IoHandler};
#[rpc]
pub trait ProofRpc {
    /// Adds two numbers and returns a result
    #[rpc(name = "add")]
    fn add(&self, a: u64, b: u64) -> Result<u64>;
}


pub struct ProofImpl{}

impl ProofRpc for ProofImpl {
    fn add(&self, a: u64, b: u64) -> Result<u64> {
        Ok(a + b)
    }
}

pub fn register(io: &mut IoHandler)  {
    let proof_impl = ProofImpl{};
    io.extend_with(proof_impl.to_delegate());
}