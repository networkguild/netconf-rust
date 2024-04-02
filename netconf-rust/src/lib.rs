use error::{Error, Result};
use message::*;
use quick_xml::de::from_str;
use std::str::FromStr;
use transport::Transport;

pub mod error;
mod framer;
pub mod message;
pub mod transport;

pub struct Connection {
    pub(crate) transport: Box<dyn Transport + Send + 'static>,

    session_id: Option<u64>,
}

impl Connection {
    pub async fn new<T>(transport: T) -> Result<Connection>
    where
        T: Transport + 'static,
    {
        let mut conn = Connection {
            transport: Box::from(transport),
            session_id: None,
        };
        conn.session_id = conn.hello().await?;
        Ok(conn)
    }

    pub fn session_id(&self) -> u64 {
        self.session_id.unwrap_or(0)
    }

    async fn hello(&mut self) -> Result<Option<u64>> {
        let hello = Hello::new();
        self.transport.write_xml(&hello.to_string()).await?;
        let response = self.transport.read_xml().await?;
        log::trace!("Hello:\n{}", response.trim());

        let hello: Hello = from_str(&response).unwrap();
        if hello.has_capability("urn:ietf:params:netconf:base:1.1".to_string()) {
            self.transport.upgrade();
        }
        Ok(hello.session_id())
    }

    pub async fn get_config(&mut self, datastore: &str) -> Result<String> {
        let get_config = Rpc::new(RpcContent::GetConfig {
            source: Source {
                datastore: Datastore::from_str(datastore)?,
            },
            filter: None,
        });
        self.transport.write_xml(&get_config.to_string()).await?;
        let response = self.transport.read_xml().await?;
        let response = response.trim();
        log::trace!("Reply:\n{}", response);

        let reply: RpcReply = from_str(response).unwrap();
        if reply.has_errors() {
            Err(Error::Netconf(reply))
        } else {
            Ok(response.to_string())
        }
    }

    pub async fn close_session(&mut self) -> Result<()> {
        let close_session = Rpc::new(RpcContent::CloseSession);
        self.transport.write_xml(&close_session.to_string()).await?;
        let response = self.transport.read_xml().await?;
        log::trace!("Reply:\n{}", response.trim());

        let reply: RpcReply = from_str(&response).unwrap();
        if reply.has_errors() {
            Err(Error::Netconf(reply))
        } else {
            Ok(())
        }
    }
}
