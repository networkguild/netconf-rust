use crate::error::{Error, Result};
use crate::framer::Framer;
use crate::transport::Transport;
use ssh2::{Channel, Session};
use std::io;
use std::net::TcpStream;

pub struct SSHTransport {
    session: Session,
    channel: Channel,
    framer: Framer,
}

impl SSHTransport {
    pub fn dial_session(session: Session) -> Result<SSHTransport> {
        connect_internal(session)
    }

    pub fn dial(addr: &str, user_name: &str, password: &str) -> Result<SSHTransport> {
        let stream = TcpStream::connect(addr)?;
        let mut sess = Session::new()?;
        sess.set_timeout(10_000);
        sess.set_tcp_stream(stream);
        sess.handshake()?;

        sess.userauth_password(user_name, password)?;
        connect_internal(sess)
    }
}

impl Transport for SSHTransport {
    fn execute_rpc(&mut self, rpc: &str) -> Result<String> {
        self.framer.write_xml(rpc, &mut self.channel)?;
        self.framer.read_xml(&mut self.channel)
    }

    fn close(&mut self) -> Result<()> {
        self.channel.send_eof()?;
        self.channel.wait_eof()?;
        self.channel.close()?;
        self.channel.wait_close()?;
        self.session
            .disconnect(Some(ssh2::ByApplication), "Shutdown", None)?;
        Ok(())
    }

    fn upgrade(&mut self) {
        self.framer.upgrade();
    }
}

fn connect_internal(session: Session) -> Result<SSHTransport> {
    if session.authenticated() {
        let mut channel = session.channel_session()?;
        channel.subsystem("netconf")?;
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
