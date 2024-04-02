use crate::message;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Ssh(#[from] async_ssh2_lite::Error),
    #[error("remote procedure call failed:\n{0}")]
    Netconf(#[from] message::RpcReply),
    #[error("unknown datastore {}, (expected {:?})", unknown, expected)]
    UnknownDatastore {
        expected: Vec<String>,
        unknown: String,
    },
    #[error("malformed message chunk")]
    MalformedChunk,
}
