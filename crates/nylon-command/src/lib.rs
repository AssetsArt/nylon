mod service;

use clap::{Parser, Subcommand};
use service::ServiceCommands;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    #[command(name = "service", short_flag = 's')]
    #[command(about = "Manage the proxy daemon service (install, start, stop, etc.)")]
    #[command(subcommand)]
    Service(ServiceCommands),

    // #[command(name = "proxy", short_flag = 'p')]
    // #[command(about = "Configure the proxy server")]
    // #[command(subcommand)]
    // Proxy(ProxyCommands),

    // run with no command
    #[command(name = "run")]
    #[command(about = "Run the proxy server with a config file")]
    Run {
        #[arg(long, short = 'c', default_value = "/etc/nylon/config.yaml")]
        #[arg(help = "Path to the config file example: /etc/nylon/config.yaml")]
        config: String,
    },
}

pub fn parse() -> Cli {
    Cli::parse()
}
