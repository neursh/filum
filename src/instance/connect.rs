use std::net::SocketAddr;

use base64::{ prelude::BASE64_STANDARD, Engine };
use colored::Colorize;
use iroh::NodeId;

use crate::{ instance::{ contact, tcp }, structs::Protocol, utils::display_info };

pub async fn establish(node: String, output: String, protocol: Protocol) {
    let translate: Vec<&str> = node.split(".").collect();

    if translate.len() != 2 {
        println!("{} {}", ">".red(), "Invalid node ID.".red().bold());
        return;
    }

    let nodeid: [u8; 32] = BASE64_STANDARD.decode(translate[0]).unwrap().try_into().unwrap();
    let alpn = translate[1].as_bytes().to_owned();
    let output_socket: SocketAddr = output.parse().unwrap();

    if nodeid.len() != 32 {
        println!(
            "{} {}",
            ">".red(),
            "Node ID from the hosting node does not match with the standard key.".red().bold()
        );
        return;
    }

    let addr_log = format!("{} :: ", output_socket);

    let endpoint = match contact::endpoint(addr_log.clone(), nodeid.clone(), alpn.clone()).await {
        Ok(endpoint) => endpoint,
        Err(_) => {
            println!("{} Can't connect to the host endpoint", ">".red());
            return;
        }
    };

    display_info::print(&endpoint.0, NodeId::from_bytes(&nodeid).unwrap(), &addr_log);
    println!("{}{}", addr_log.green(), format!("Proxy server: {}", output_socket).bright_cyan());

    match protocol {
        Protocol::Tcp => tcp::connection_bridge(output_socket, endpoint).await,
        Protocol::Udp => todo!(), //udp::connection_bridge(output_socket, endpoint).await,
    }
}
