mod host;
mod structs;
mod instance;

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
    Instance {
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
        /// IP and port. Ex: "0.0.0.0:7141".
        output: String,

        /// A provided node ID from host.
        node: String,
    },
    Udp {
        /// IP and port. Ex: "0.0.0.0:7141".
        output: String,

        /// A provided node ID from host.
        node: String,
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

        Commands::Instance { protocol: ClientProtocolOptions::Tcp { node, output } } =>
            instance::connect::establish(node, output, Protocol::Tcp).await,
        Commands::Instance { protocol: ClientProtocolOptions::Udp { node, output } } =>
            instance::connect::establish(node, output, Protocol::Udp).await,
    }
}
