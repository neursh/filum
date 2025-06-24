mod host;
mod structs;
mod client;

use clap::{ Parser, Subcommand };

use crate::structs::Protocol;

/// Filum | Localhost over P2P | https://github.com/neursh/filum
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Proxy a P2P service on your local network port.
    Host {
        #[command(subcommand)]
        protocol: HostProtocolOptions,
    },

    /// Connect to a P2P service.
    Client {
        #[command(subcommand)]
        protocol: ClientProtocolOptions,
    },
}

#[derive(Subcommand)]
enum HostProtocolOptions {
    Tcp {
        /// IP and port. Ex: "0.0.0.0:7141"
        source: String,
    },
    Udp {
        /// IP and port. Ex: "0.0.0.0:7141"
        source: String,
    },
}

#[derive(Subcommand)]
enum ClientProtocolOptions {
    Tcp {
        /// A provided node ID from host.
        node: String,

        /// Destination port to use on client, leave empty to let the program to select a random one.
        #[arg(default_value("0"))]
        port: u16,
    },
    Udp {
        /// A provided node ID from host.
        node: String,

        /// Destination port to use on client, leave empty to let the program to select a random one.
        #[arg(default_value("0"))]
        port: u16,
    },
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    match args.command {
        Commands::Host { protocol: HostProtocolOptions::Tcp { source } } =>
            host::create::create_host(source, Protocol::Tcp).await,
        Commands::Host { protocol: HostProtocolOptions::Udp { source } } =>
            host::create::create_host(source, Protocol::Udp).await,

        Commands::Client { protocol: ClientProtocolOptions::Tcp { node, port } } =>
            client::connect::establish(node, port, Protocol::Tcp).await,
        Commands::Client { protocol: ClientProtocolOptions::Udp { node, port } } =>
            client::connect::establish(node, port, Protocol::Udp).await,
    }
}
