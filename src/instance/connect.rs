use std::net::SocketAddr;

use base64::{ prelude::BASE64_STANDARD, Engine };
use colored::Colorize;

use crate::{ instance::{ tcp, udp }, structs::Protocol };

pub async fn establish(node: String, output: String, protocol: Protocol) {
    let translate: Vec<&str> = node.split(".").collect();

    let nodeid = BASE64_STANDARD.decode(translate[0]).unwrap();
    let alpn = translate[1].as_bytes().to_owned();
    let output_socket: SocketAddr = output.parse().unwrap();

    if translate.len() != 2 {
        println!("{} {}", ">".red(), "Invalid node ID.".red().bold());
        return;
    }

    if nodeid.len() != 32 {
        println!(
            "{} {}",
            ">".red(),
            "Node ID from the hosting node does not match with the standard key.".red().bold()
        );
        return;
    }

    let nodeid = nodeid[..32].try_into().unwrap();

    println!("{} {}", ">".green(), format!("Proxy server: {}", output_socket).bright_cyan());
    match protocol {
        Protocol::Tcp => tcp::connection_bridge(output_socket, nodeid, alpn).await,
        Protocol::Udp => udp::connection_bridge(output_socket, nodeid, alpn).await,
    }
}
