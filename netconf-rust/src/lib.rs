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
    skip_errors: bool,
}

impl Connection {
    pub fn new<T>(transport: T) -> Result<Connection>
    where
        T: Transport + 'static,
    {
        let mut conn = Connection {
            transport: Box::from(transport),
            session_id: None,
            skip_errors: false,
        };
        conn.session_id = conn.hello()?;
        Ok(conn)
    }

    pub fn set_skip_errors(&mut self) {
        self.skip_errors = true
    }

    pub fn session_id(&self) -> u64 {
        self.session_id.unwrap_or(0)
    }

    fn hello(&mut self) -> Result<Option<u64>> {
        let hello = Hello::new();
        let response = self.transport.execute_rpc(&hello.to_string())?;
        log::trace!("Hello:\n{}", response);

        let hello: Hello = from_str(&response)?;
        if hello.has_capability("urn:ietf:params:netconf:base:1.1".to_string()) {
            self.transport.upgrade();
        }
        Ok(hello.session_id())
    }

    pub fn get_config(&mut self, datastore: &str) -> Result<String> {
        let get_config = Rpc::new(RpcContent::GetConfig {
            source: Source {
                datastore: Datastore::from_str(datastore)?,
            },
            filter: None,
        });
        let response = self.transport.execute_rpc(&get_config.to_string())?;
        log::trace!("Reply:\n{}", response);

        if !self.skip_errors {
            let reply: RpcReply = from_str(&response)?;
            if reply.has_errors() {
                return Err(Error::Netconf(reply));
            }
        }
        Ok(response.to_string())
    }

    pub fn close_session(&mut self) -> Result<()> {
        let close_session = Rpc::new(RpcContent::CloseSession);
        let response = self.transport.execute_rpc(&close_session.to_string())?;
        log::trace!("Reply:\n{}", response.trim());

        let reply: RpcReply = from_str(&response)?;
        if reply.has_errors() {
            Err(Error::Netconf(reply))
        } else {
            Ok(())
        }
    }
}
