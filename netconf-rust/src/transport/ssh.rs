use crate::error::{Error, Result};
use crate::framer::Framer;
use crate::transport::Transport;
use async_ssh2_lite::{ssh2, AsyncChannel, AsyncSession, SessionConfiguration, TokioTcpStream};
use async_trait::async_trait;
use ssh2::{Channel, Session};
use std::io;

pub struct SSHTransport {
    #[cfg(feature = "async")]
    session: AsyncSession<TokioTcpStream>,
    #[cfg(feature = "blocking")]
    channel: Session,
    #[cfg(feature = "async")]
    channel: AsyncChannel<TokioTcpStream>,
    #[cfg(feature = "blocking")]
    channel: Channel,
    framer: Framer,
}

impl SSHTransport {
    #[cfg(feature = "async")]
    pub async fn dial_session(session: AsyncSession<TokioTcpStream>) -> Result<SSHTransport> {
        connect_internal(session).await
    }

    #[cfg(feature = "blocking")]
    pub async fn dial_session(session: Session) -> Result<SSHTransport> {
        connect_internal(session).await
    }

    #[cfg(feature = "async")]
    pub async fn dial(addr: &str, user_name: &str, password: &str) -> Result<SSHTransport> {
        let stream = TokioTcpStream::connect(addr).await?;
        let mut configuration = SessionConfiguration::new();
        configuration.set_timeout(10000);
        let mut sess = AsyncSession::new(stream, configuration)?;
        sess.handshake().await?;

        sess.userauth_password(user_name, password).await?;
        connect_internal(sess).await
    }
}

#[async_trait]
#[cfg(feature = "tokio")]
impl Transport for SSHTransport {
    async fn read_xml(&mut self) -> Result<String> {
        self.framer.read_xml(&mut self.channel).await
    }

    async fn write_xml(&mut self, data: &str) -> Result<()> {
        self.framer.write_xml(data, &mut self.channel).await
    }

    async fn close(&mut self) -> Result<()> {
        self.channel.send_eof().await?;
        self.channel.wait_eof().await?;
        self.channel.close().await?;
        self.channel.wait_close().await?;
        self.session
            .disconnect(Some(ssh2::ByApplication), "Shutdown", None)
            .await?;
        Ok(())
    }

    fn upgrade(&mut self) {
        self.framer.upgrade();
    }
}

async fn connect_internal(session: AsyncSession<TokioTcpStream>) -> Result<SSHTransport> {
    if session.authenticated() {
        let mut channel = session.channel_session().await?;
        channel.subsystem("netconf").await?;
        let transport = SSHTransport {
            session,
            channel,
            framer: Framer::new(),
        };
        Ok(transport)
    } else {
        Err(Error::Io(io::Error::last_os_error()))
    }
}
