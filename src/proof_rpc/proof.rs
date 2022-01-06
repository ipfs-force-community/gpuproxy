use jsonrpc_core::Result;
use jsonrpc_derive::rpc;

build_rpc_trait! {
    pub trait ProofRpc {
        /// Adds two numbers and returns a result
        #[rpc(name = "add")]
        fn add(&self, a: u64, b: u64) -> Result<u64>;
    }
}

pub struct ProofImpl{

}

impl ProofRpc for ProofImpl {
    fn add(&self, a: u64, b: u64) -> Result<u64> {
        Ok(a + b)
    }
}