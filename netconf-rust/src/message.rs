#![allow(dead_code)]
use crate::error;
use quick_xml::se::Serializer;
use serde::Serialize;
use serde_derive::{Deserialize, Serialize};
use std::fmt;
use std::fmt::Display;
use std::str::FromStr;
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename(serialize = "hello"))]
pub struct Hello {
    #[serde(rename = "@xmlns")]
    xmlns: String,
    capabilities: Capabilities,
    #[serde(rename = "session-id", skip_serializing_if = "Option::is_none")]
    session_id: Option<u64>,
}

impl Hello {
    pub fn new() -> Hello {
        Hello {
            xmlns: "urn:ietf:params:xml:ns:netconf:base:1.0".to_string(),
            session_id: None,
            capabilities: Capabilities {
                capability: vec![
                    "urn:ietf:params:netconf:base:1.0".to_string(),
                    "urn:ietf:params:netconf:base:1.1".to_string(),
                ],
            },
        }
    }

    pub fn capabilities(&self) -> Vec<String> {
        self.capabilities
            .capability
            .iter()
            .map(|capability| capability.to_string())
            .collect()
    }

    pub fn has_capability(&self, capability: String) -> bool {
        self.capabilities.capability.contains(&capability)
    }

    pub fn session_id(&self) -> Option<u64> {
        self.session_id
    }
}

impl Display for Hello {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut buffer = String::with_capacity(206);
        let ser = Serializer::new(&mut buffer);
        self.serialize(ser).unwrap();
        write!(f, "{}", buffer)
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Capabilities {
    capability: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename(serialize = "rpc"))]
pub struct Rpc {
    #[serde(rename = "@xmlns")]
    xmlns: String,
    #[serde(rename = "@message-id")]
    message_id: String,
    #[serde(rename = "$value")]
    content: RpcContent,
}

impl Rpc {
    pub fn new(content: RpcContent) -> Rpc {
        Rpc {
            xmlns: "urn:ietf:params:xml:ns:netconf:base:1.0".to_string(),
            message_id: Uuid::new_v4().to_string(),
            content,
        }
    }
}

impl Display for Rpc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut buffer = String::with_capacity(256);
        let ser = Serializer::new(&mut buffer);
        self.serialize(ser).unwrap();
        write!(f, "{}", buffer)
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RpcContent {
    CloseSession,
    KillSession,
    GetConfig {
        source: Source,
        #[serde(rename = "filter", skip_serializing_if = "Option::is_none")]
        filter: Option<Filter>,
    },
}

#[derive(Debug, Serialize)]
pub struct Source {
    #[serde(rename = "$value")]
    pub datastore: Datastore,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Datastore {
    Candidate,
    Running,
    Startup,
    Url(String),
}

impl FromStr for Datastore {
    type Err = error::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let datastore = s.to_lowercase();
        match datastore.as_str() {
            "running" => Ok(Datastore::Running),
            "candidate" => Ok(Datastore::Candidate),
            "startup" => Ok(Datastore::Startup),
            _ => {
                if datastore.starts_with("http")
                    || datastore.starts_with("file")
                    || datastore.starts_with("ftp")
                {
                    Ok(Datastore::Url(datastore))
                } else {
                    Err(error::Error::UnknownDatastore {
                        expected: vec![
                            "running".to_string(),
                            "candidate".to_string(),
                            "startup".to_string(),
                            "ftp|http|file".to_string(),
                        ],
                        unknown: datastore,
                    })
                }
            }
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Filter {
    #[serde(rename = "@type")]
    filter_type: String,
    filter: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", rename(serialize = "rpc-reply"))]
pub struct RpcReply {
    #[serde(rename = "@message-id")]
    message_id: String,
    #[serde(default)]
    rpc_error: Vec<Error>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ok: Option<()>,
}

impl RpcReply {
    pub fn has_errors(&self) -> bool {
        !self.rpc_error.is_empty()
    }
}

impl Display for RpcReply {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut buffer = String::new();
        let ser = Serializer::new(&mut buffer);
        self.serialize(ser).unwrap();
        write!(f, "{}", buffer)
    }
}

impl std::error::Error for RpcReply {}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename = "rpc-error", rename_all = "kebab-case")]
pub struct Error {
    error_severity: ErrorSeverity,
    error_type: ErrorType,
    error_tag: ErrorTag,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_app_tag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_info: Option<ErrorInfo>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
enum ErrorType {
    Transport,
    Rpc,
    Protocol,
    App,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
enum ErrorSeverity {
    Error,
    Warning,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
enum ErrorTag {
    InUse,
    InvalidValue,
    TooBig,
    MissingAttribute,
    BadAttribute,
    UnknownAttribute,
    MissingElement,
    BadElement,
    UnknownElement,
    UnknownNamespace,
    AccessDenied,
    LockDenied,
    ResourceDenied,
    RollbackFailed,
    DataExists,
    DataMissing,
    OperationNotSupported,
    OperationFailed,
    PartialOperation,
    MalformedMessage,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
struct ErrorInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    bad_element: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bad_attribute: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bad_namespace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ok_element: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    err_element: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    noop_element: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    session_id: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use quick_xml::de::from_str;

    #[test]
    fn test_deserialize_reply_with_errors() {
        let reply = r#"
<rpc-reply message-id="67d83d6b-1f0b-47fb-8fdf-2cfc3fb2a371" xmlns="urn:ietf:params:xml:ns:netconf:base:1.0">
  <rpc-error>
    <error-type>protocol</error-type>
    <error-tag>bad-element</error-tag>
    <error-severity>error</error-severity>
    <error-message>Element is not valid in the specified context.</error-message>
    <error-info>
      <bad-element>startu</bad-element>
    </error-info>
  </rpc-error>
  <rpc-error>
    <error-type>app</error-type>
    <error-tag>bad-element</error-tag>
    <error-severity>error</error-severity>
    <error-message>Element is not valid in the specified context.</error-message>
    <error-info>
      <bad-element>startu</bad-element>
    </error-info>
  </rpc-error>
</rpc-reply>
"#.trim();

        let reply: RpcReply = from_str(reply).unwrap();
        println!("{:?}", reply);
    }

    #[test]
    fn test_serialize_hello() {
        let expected = r#"
<hello xmlns="urn:ietf:params:xml:ns:netconf:base:1.0">
  <capabilities>
    <capability>urn:ietf:params:netconf:base:1.0</capability>
    <capability>urn:ietf:params:netconf:base:1.1</capability>
  </capabilities>
</hello>
"#
        .trim()
        .to_string();

        let hello = Hello {
            xmlns: "urn:ietf:params:xml:ns:netconf:base:1.0".to_string(),
            session_id: None,
            capabilities: Capabilities {
                capability: vec![
                    "urn:ietf:params:netconf:base:1.0".to_string(),
                    "urn:ietf:params:netconf:base:1.1".to_string(),
                ],
            },
        };

        assert_eq!(hello.to_string(), expected);
    }

    #[test]
    fn test_serialize_close_session() {
        let expected = r#"
<rpc xmlns="urn:ietf:params:xml:ns:netconf:base:1.0" message-id="c1be0e7f-3cbc-413f-8aa8-18ed663221d4">
  <close-session/>
</rpc>
"#.trim().to_string();

        let close_session = Rpc {
            xmlns: "urn:ietf:params:xml:ns:netconf:base:1.0".to_string(),
            message_id: "c1be0e7f-3cbc-413f-8aa8-18ed663221d4".to_string(),
            content: RpcContent::CloseSession,
        };
        assert_eq!(close_session.to_string(), expected.trim());
    }

    #[test]
    fn test_serialize_kill_session() {
        let expected = r#"
<rpc xmlns="urn:ietf:params:xml:ns:netconf:base:1.0" message-id="c1be0e7f-3cbc-413f-8aa8-18ed663221d4">
  <kill-session/>
</rpc>
"#.trim().to_string();

        let close_session = Rpc {
            xmlns: "urn:ietf:params:xml:ns:netconf:base:1.0".to_string(),
            message_id: "c1be0e7f-3cbc-413f-8aa8-18ed663221d4".to_string(),
            content: RpcContent::KillSession,
        };
        assert_eq!(close_session.to_string(), expected.trim());
    }

    #[test]
    fn test_serialize_get_config() {
        let expected = r#"
<rpc xmlns="urn:ietf:params:xml:ns:netconf:base:1.0" message-id="c1be0e7f-3cbc-413f-8aa8-18ed663221d4">
  <get-config>
    <source>
      <running/>
    </source>
  </get-config>
</rpc>
"#
    .trim()
    .to_string();

        let close_session = Rpc {
            xmlns: "urn:ietf:params:xml:ns:netconf:base:1.0".to_string(),
            message_id: "c1be0e7f-3cbc-413f-8aa8-18ed663221d4".to_string(),
            content: RpcContent::GetConfig {
                source: Source {
                    datastore: Datastore::Running,
                },
                filter: None,
            },
        };
        assert_eq!(close_session.to_string(), expected.trim());
    }
}
