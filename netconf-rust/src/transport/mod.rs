use crate::error::Result;
use async_trait::async_trait;

pub mod ssh;

/// Trait for NETCONF transport
#[async_trait]
pub trait Transport: Send {
    async fn read_xml(&mut self) -> Result<String>;
    async fn write_xml(&mut self, data: &str) -> Result<()>;
    async fn close(&mut self) -> Result<()>;
    fn upgrade(&mut self);
}
