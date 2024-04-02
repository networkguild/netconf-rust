use crate::error::Result;

pub mod ssh;

/// Trait for NETCONF transport
pub trait Transport: Send {
    fn execute_rpc(&mut self, rpc: &str) -> Result<String>;
    fn close(&mut self) -> Result<()>;
    fn upgrade(&mut self);
}
