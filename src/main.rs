mod host;
mod structs;
mod instance;
mod utils;

use clap::{ Parser, Subcommand };
use colored::Colorize;

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
    let args = match Args::try_parse() {
        Ok(args) => args,
        Err(_) => {
            println!("{}", "No arguments provided. Asking manually...".yellow());
            println!("{}", "Let's get you started.".green());

            let role = inquire::Select
                ::new(
                    "What role do you want Filum to run on?",
                    vec!["Host".to_owned(), "Instance".to_owned()]
                )
                .prompt()
                .unwrap();

            let protocol = inquire::Select
                ::new("What protocol?", vec!["TCP".to_owned(), "UDP".to_owned()])
                .prompt()
                .unwrap();

            let attached_addr = inquire::Text
                ::new("What local address do you want Filum to attach to?")
                .prompt()
                .unwrap();

            match role.as_str() {
                "Host" => {
                    Args {
                        command: Commands::Host {
                            protocol: match protocol.as_str() {
                                "TCP" => HostProtocolOptions::Tcp { source: attached_addr },
                                "UDP" => HostProtocolOptions::Udp { source: attached_addr },
                                _ => {
                                    return;
                                }
                            },
                        },
                    }
                }
                "Instance" => {
                    let node = inquire::Text
                        ::new("ID of the node you want to connect to:")
                        .prompt()
                        .unwrap();
                    Args {
                        command: Commands::Instance {
                            protocol: match protocol.as_str() {
                                "TCP" => ClientProtocolOptions::Tcp { output: attached_addr, node },
                                "UDP" => ClientProtocolOptions::Udp { output: attached_addr, node },
                                _ => {
                                    return;
                                }
                            },
                        },
                    }
                }
                _ => {
                    return;
                }
            }
        }
    };

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

    println!("\n{}", "Press Enter to exit the program...".bold());
    let _ = std::io::stdin().read_line(&mut String::new()).unwrap();
}
