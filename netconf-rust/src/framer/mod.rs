use crate::error::{Error, Result};
use memmem::{Searcher, TwoWaySearcher};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

const NETCONF_1_0_TERMINATOR: &str = "]]>]]>";
const NETCONF_1_1_TERMINATOR: &str = "##";

/// Trait for NETCONF message framing
/// See [RFC6242](https://tools.ietf.org/html/rfc6242#section-4.1)
pub(crate) struct Framer {
    read_buffer: Vec<u8>,
    upgraded: bool,
}

impl Framer {
    pub(crate) fn new() -> Framer {
        Framer {
            read_buffer: Vec::new(),
            upgraded: false,
        }
    }

    pub(crate) fn upgrade(&mut self) {
        self.upgraded = true;
    }

    pub(crate) async fn read_xml<R>(&mut self, mut from: R) -> Result<String>
    where
        R: AsyncRead + Unpin,
    {
        if self.upgraded {
            loop {
                let chunk_size: u32 = self.read_header(&mut from).await?;
                if chunk_size == 0 {
                    break;
                }
                let mut buffer = vec![0u8; chunk_size as usize];
                from.read(&mut buffer).await?;
                self.read_buffer.extend(&buffer);
            }
            let response = String::from_utf8(self.read_buffer.to_vec()).unwrap();
            self.read_buffer.drain(..);
            Ok(response)
        } else {
            let mut buffer = [0u8; 128];
            let search = TwoWaySearcher::new(NETCONF_1_0_TERMINATOR.as_bytes());
            while search.search_in(&self.read_buffer).is_none() {
                let bytes = from.read(&mut buffer).await?;
                self.read_buffer.extend(&buffer[..bytes]);
            }
            let pos = search.search_in(&self.read_buffer).unwrap();
            let resp = String::from_utf8(self.read_buffer[..pos].to_vec()).unwrap();
            self.read_buffer.drain(0..(pos + 6));
            Ok(resp.trim().to_string())
        }
    }

    pub(crate) async fn write_xml<T>(&mut self, data: &str, mut to: T) -> Result<()>
    where
        T: AsyncWrite + Unpin,
    {
        let data = data.trim();
        if self.upgraded {
            let data =
                format!("\n#{}\n{}\n{}\n", data.len(), data, NETCONF_1_1_TERMINATOR).into_bytes();
            to.write_all(&data).await?;
        } else {
            let data = format!("{}{}", data.trim(), NETCONF_1_0_TERMINATOR).into_bytes();
            to.write_all(&data).await?;
        }
        Ok(())
    }

    async fn read_header<R>(&mut self, mut from: R) -> Result<u32>
    where
        R: AsyncRead + Unpin,
    {
        let mut buffer = [0u8; 2];
        from.read_exact(&mut buffer).await?;
        if buffer[0] != b'\n' || buffer[1] != b'#' {
            return Err(Error::MalformedChunk);
        }

        let mut chunk_size: u32 = 0;
        let mut last_read: u8;
        loop {
            let mut buffer = [0u8; 1];
            from.read_exact(&mut buffer).await?;
            last_read = buffer[0];
            if last_read == b'#' {
                continue;
            }
            if last_read == b'\n' {
                return Ok(chunk_size);
            }
            if last_read < b'0' || last_read > b'9' {
                return Err(Error::MalformedChunk);
            }
            chunk_size = chunk_size * 10 + u32::from(last_read - b'0');
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::io::Cursor;

    #[tokio::test]
    async fn test_chunked_framer() {
        let mut framer = Framer::new();
        framer.upgrade();

        let rpc_error = r#"
#38
<?xml version="1.0" encoding="UTF-8"?>
#1


#10
<rpc-reply
#50
 message-id="8ddd59e5-96fc-4a55-a75f-a3fae2d9f712"
#48
 xmlns="urn:ietf:params:xml:ns:netconf:base:1.0"
#1
>
#1


#14
    <rpc-error
#1
>
#1


#41
        <error-type>protocol</error-type>
#1


#42
        <error-tag>bad-element</error-tag>
#1


#46
        <error-severity>error</error-severity>
#1


#22
        <error-message
#1
>
#1


#58
            Element is not valid in the specified context.
#1


#24
        </error-message>
#1


#19
        <error-info
#1
>
#1


#45
            <bad-element>startu</bad-element>
#1


#21
        </error-info>
#1


#16
    </rpc-error>
#1


#12
</rpc-reply>
##

"#
        .to_string();
        let channel = Cursor::new(rpc_error.into_bytes());
        let (read, _) = tokio::io::split(channel);
        let resp = framer.read_xml(read).await.unwrap();
        let expected = r#"
<?xml version="1.0" encoding="UTF-8"?>
<rpc-reply message-id="8ddd59e5-96fc-4a55-a75f-a3fae2d9f712" xmlns="urn:ietf:params:xml:ns:netconf:base:1.0">
    <rpc-error>
        <error-type>protocol</error-type>
        <error-tag>bad-element</error-tag>
        <error-severity>error</error-severity>
        <error-message>
            Element is not valid in the specified context.
        </error-message>
        <error-info>
            <bad-element>startu</bad-element>
        </error-info>
    </rpc-error>
</rpc-reply>
"#;
        assert_eq!(resp, expected.trim());
    }

    #[tokio::test]
    async fn test_eof_framer() {
        let mut framer = Framer::new();
        let rpc_error = r#"
<?xml version="1.0" encoding="UTF-8"?>
<rpc-reply message-id="8ddd59e5-96fc-4a55-a75f-a3fae2d9f712" xmlns="urn:ietf:params:xml:ns:netconf:base:1.0">
    <rpc-error>
        <error-type>protocol</error-type>
        <error-tag>bad-element</error-tag>
        <error-severity>error</error-severity>
        <error-message>
            Element is not valid in the specified context.
        </error-message>
        <error-info>
            <bad-element>startu</bad-element>
        </error-info>
    </rpc-error>
</rpc-reply>
]]>]]>"#
            .to_string();
        let channel = Cursor::new(rpc_error.into_bytes());
        let (read, _) = tokio::io::split(channel);
        let resp = framer.read_xml(read).await.unwrap();
        let expected = r#"
<?xml version="1.0" encoding="UTF-8"?>
<rpc-reply message-id="8ddd59e5-96fc-4a55-a75f-a3fae2d9f712" xmlns="urn:ietf:params:xml:ns:netconf:base:1.0">
    <rpc-error>
        <error-type>protocol</error-type>
        <error-tag>bad-element</error-tag>
        <error-severity>error</error-severity>
        <error-message>
            Element is not valid in the specified context.
        </error-message>
        <error-info>
            <bad-element>startu</bad-element>
        </error-info>
    </rpc-error>
</rpc-reply>
"#;
        assert_eq!(resp, expected.trim());
    }
}
