use clap::{Args, Parser, Subcommand};
use env_logger::{Builder, Env, Target};
use netconf_rust::error::Result;
use netconf_rust::Connection;
use ssh::Host;
use ssh2_config::HostParams;
use std::env;
use std::thread;
use std::time::Instant;

mod ssh;

const ABOUT: &str = "Netconf cli tool written in Rust\nUse NETCONF_LOG to set log filter and level";

#[derive(Debug, Parser)]
#[command(version, about = "Netconf cli tool", long_about = ABOUT)]
#[command(name = "netconf")]
struct Cli {
    #[arg(short, long, global = true, help = "Enables debug level logging")]
    debug: bool,
    #[arg(short, long, global = true, help = "Enables trace level logging")]
    trace: bool,

    #[arg(
        long,
        global = true,
        value_delimiter = ',',
        env = "NETCONF_HOST",
        help = "Host(s) to connect. Value can include port, eg. 172.30.15.1:22. Default port is 830"
    )]
    host: Vec<String>,
    #[arg(
        short,
        long,
        global = true,
        env = "NETCONF_USERNAME",
        help = "Default usename for all host, ssh config value will override"
    )]
    username: Option<String>,
    #[arg(
        short,
        long,
        global = true,
        env = "NETCONF_PASSWORD",
        help = "Password for all hosts",
        hide_env_values = true
    )]
    password: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[command(about = "Get rpc with custom filters")]
    Get(GetConfigArgs),
    #[command(about = "Get-config rpc from specific datastore")]
    GetConfig(GetConfigArgs),
    #[command(about = "Edit-config rpc")]
    EditConfig(EditConfigArgs),
}

#[derive(Debug, Args, Clone, Default)]
struct GetConfigArgs {
    #[arg(short, long, default_value = "running")]
    source: String,
}

#[derive(Debug, Args, Clone, Default)]
struct EditConfigArgs {
    #[arg(short, long, default_value = "running")]
    source: String,
}

fn init_logging() {
    let env = Env::default().filter_or("NETCONF_LOG", "info");
    let mut builder = Builder::new();
    builder.target(Target::Stdout);
    builder.parse_env(env);
    builder.init();
}

fn main() {
    let cli = Cli::parse();
    if cli.debug {
        env::set_var("NETCONF_LOG", "debug");
    }
    if cli.trace {
        env::set_var("NETCONF_LOG", "trace");
    }
    init_logging();

    let config = ssh::read_config();
    let mut hosts = Vec::new();
    for address in cli.host.iter() {
        let command = match &cli.command {
            Commands::GetConfig(args) => Commands::GetConfig(args.clone()),
            Commands::Get(args) => Commands::Get(args.clone()),
            Commands::EditConfig(args) => Commands::EditConfig(args.clone()),
        };
        hosts.push(Host::new(
            address,
            cli.username.clone(),
            cli.password.clone(),
            command,
        ));
    }

    let mut handles = vec![];
    for mut host in hosts.into_iter() {
        let params = match &config {
            Some(p) => p.query(host.address()),
            None => HostParams::default(),
        };

        let start_time = Instant::now();
        let task = thread::spawn(move || match host.connect(&params) {
            Ok(session) => {
                let ssh =
                    netconf_rust::transport::ssh::SSHTransport::dial_session(session).unwrap();
                log::info!(target: &host.address(), "Connected to host");
                let mut connection = Connection::new(ssh).unwrap();
                log::debug!(
                    target: &host.address(),
                    "Started Netconf session with session-id: {}",
                    connection.session_id()
                );

                match &host.command {
                    Commands::GetConfig(args) => {
                        run_get_config(&host.address(), args, &mut connection).unwrap();
                    }
                    Commands::Get(args) => {
                        run_get(&host.address(), args, &mut connection).unwrap();
                    }
                    Commands::EditConfig(_args) => {
                        log::warn!("Edit-config not implemented yet");
                    }
                };
                log::info!(target: &host.address(), "Operation took: {:.3}s", start_time.elapsed().as_secs_f32());
            }
            Err(err) => {
                log::error!(target: &host.address(), "Could not connect to host, error: {err}");
            }
        });
        handles.push(task);
    }

    for i in handles {
        match i.join() {
            Ok(_) => {}
            Err(err) => {
                log::error!("Task error: {:?}", err);
            }
        };
    }
}

fn run_get(address: &str, args: &GetConfigArgs, connection: &mut Connection) -> Result<()> {
    match connection.get_config(&args.source) {
        Ok(resp) => {
            log::info!("Get rpc success");
            log::trace!(target: address, "Response:\n{}", resp.trim());
        }
        Err(err) => {
            log::error!(target: address, "Get error: {}", err);
        }
    };
    connection.close_session().unwrap();
    Ok(())
}

fn run_get_config(address: &str, args: &GetConfigArgs, connection: &mut Connection) -> Result<()> {
    match connection.get_config(&args.source) {
        Ok(resp) => {
            log::info!("Get-config rpc success");
            log::trace!(target: address, "Response:\n{}", resp.trim());
        }
        Err(err) => {
            log::error!(target: address, "Get-config error: {}", err);
        }
    };
    connection.close_session().unwrap();
    Ok(())
}
