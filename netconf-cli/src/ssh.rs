use crate::Commands;
use dirs::home_dir;
use ssh2::{MethodType, Session};
use ssh2_config::{HostParams, ParseRule, SshConfig};
use std::fs::File;
use std::io;
use std::io::BufReader;
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::path::Path;
use std::time::Duration;

pub(crate) struct Host {
    address: String,
    port: u16,
    username: Option<String>,
    password: Option<String>,
    pub(crate) command: Commands,
}

impl Host {
    pub(crate) fn new(
        addr: &str,
        username: Option<String>,
        password: Option<String>,
        command: Commands,
    ) -> Host {
        let port: u16;
        let address: String;
        match addr.contains(':') {
            true => {
                let parts: Vec<&str> = addr.split(':').collect();
                address = parts[0].to_string();
                port = parts[1].parse().unwrap();
            }
            false => {
                address = addr.to_string();
                port = 830;
            }
        };
        Host {
            address,
            port,
            username,
            password,
            command,
        }
    }

    pub(crate) fn address(&self) -> String {
        format!("{}:{}", self.address, self.port)
    }

    pub(crate) fn connect(&mut self, params: &HostParams) -> Result<Session, io::Error> {
        let address = match params.host_name.as_deref() {
            Some(host) => {
                self.address = host.to_string();
                host
            }
            None => &self.address,
        };
        let port = params.port.unwrap_or(self.port);
        let address = format!("{}:{}", address, port);

        let socket_addresses: Vec<SocketAddr> = address.to_socket_addrs()?.collect();
        let mut tcp: Option<TcpStream> = None;
        for socket_addr in socket_addresses.iter() {
            log::debug!(target: &self.address(), "Trying to establish connection to {}", socket_addr);
            match TcpStream::connect_timeout(
                socket_addr,
                params.connect_timeout.unwrap_or(Duration::from_secs(10)),
            ) {
                Ok(stream) => {
                    log::info!(target: &self.address(), "Established connection to {}", socket_addr);
                    tcp = Some(stream);
                    break;
                }
                Err(err) => {
                    log::error!(
                        target: &self.address(),
                        "Could not establish connection to '{}': {}",
                        socket_addr,
                        err
                    );
                    continue;
                }
            }
        }
        let stream: TcpStream = match tcp {
            Some(t) => t,
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "No suitable socket address found; connection timeout",
                ));
            }
        };

        let mut session = Session::new()?;
        configure_session(&mut session, params)?;
        session.set_timeout(10_000);
        session.set_tcp_stream(stream);
        session.handshake()?;

        if params.identity_file.is_none() {
            let username = match params.user.as_ref() {
                Some(u) => {
                    log::debug!(target: &self.address(), "Using username '{}'", u);
                    u.clone()
                }
                None => self.username.clone().unwrap(),
            };
            session
                .userauth_password(username.as_str(), self.password.clone().unwrap().as_str())?;
            Ok(session)
        } else {
            let mut agent = session.agent().unwrap();
            agent.connect().unwrap();
            agent.list_identities().unwrap();

            let user = params.user.as_deref().unwrap();
            for identity in agent.identities().unwrap() {
                log::debug!(
                    target: &self.address(),
                    "Trying authentication with public key '{}'",
                    identity.comment()
                );
                match agent.userauth(user, &identity) {
                    Ok(_) => break,
                    Err(err) => {
                        log::warn!(
                            target: &self.address(),
                            "Public key '{}' authentication failed: {}",
                            identity.comment(),
                            err
                        );
                        continue;
                    }
                }
            }

            if session.authenticated() {
                Ok(session)
            } else {
                Err(io::Error::new(
                    io::ErrorKind::Other,
                    "Authentication failed, no suitable public key found",
                ))
            }
        }
    }
}

pub(crate) fn read_config() -> Option<SshConfig> {
    let mut home = home_dir().expect("Failed to get home_dir for guest OS");
    home.extend(Path::new(".ssh/config"));
    log::debug!("Trying to parse ssh configuration '{}'", home.display());

    let mut reader = match File::open(home.as_path()) {
        Ok(f) => BufReader::new(f),
        Err(err) => {
            log::warn!(
                "Could not open ssh config file '{}', disable config reading with --no-config flag: {}",
                home.display(),
                err
            );
            return None;
        }
    };
    match SshConfig::default().parse(&mut reader, ParseRule::STRICT) {
        Ok(config) => {
            log::debug!("Successfully parsed configuration");
            Some(config)
        }
        Err(err) => {
            log::error!("Failed to parse ssh configuration, error '{}'", err);
            None
        }
    }
}

fn configure_session(session: &mut Session, params: &HostParams) -> Result<(), io::Error> {
    if let Some(compress) = params.compression {
        log::debug!("Setting compression: {}", compress);
        session.set_compress(compress);
    }
    if params.tcp_keep_alive.unwrap_or(false) && params.server_alive_interval.is_some() {
        let interval = params.server_alive_interval.unwrap().as_secs() as u32;
        log::debug!("Setting keepalive interval: {} seconds", interval);
        session.set_keepalive(true, interval);
    }
    if let Some(algos) = params.kex_algorithms.as_deref() {
        session.method_pref(MethodType::Kex, algos.join(",").as_str())?;
    }
    if let Some(algos) = params.host_key_algorithms.as_deref() {
        session.method_pref(MethodType::HostKey, algos.join(",").as_str())?;
    }
    if let Some(algos) = params.ciphers.as_deref() {
        session.method_pref(MethodType::CryptCs, algos.join(",").as_str())?;
    }
    if let Some(algos) = params.mac.as_deref() {
        session.method_pref(MethodType::MacCs, algos.join(",").as_str())?;
        session.method_pref(MethodType::MacSc, algos.join(",").as_str())?;
    }
    Ok(())
}
