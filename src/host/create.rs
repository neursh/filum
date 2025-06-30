use std::net::SocketAddr;

use base64::{ prelude::BASE64_STANDARD, Engine };
use colored::Colorize;
use iroh::{ endpoint::Connection, Endpoint };
use nanoid::nanoid;

use crate::{ host::{ tcp, udp }, structs::Protocol, utils::display_info };

pub async fn create_host(source: String, protocol: Protocol) {
    let alpn = nanoid!(32);
    let endpoint = Endpoint::builder()
        .discovery_n0()
        .alpns(vec![alpn.as_bytes().to_vec()])
        .bind().await
        .unwrap();
    let nodeid = BASE64_STANDARD.encode(endpoint.node_id().as_bytes());
    let source_socket: SocketAddr = source.parse().unwrap();

    println!(
        "{} Service started, you can now share this ID to client to let them connect to {}.",
        ">".green(),
        format!("{}", source_socket).bright_cyan()
    );
    println!("ID: {}", format!("{}.{}", nodeid, alpn).bright_cyan());

    accept_handler(endpoint, &source_socket, protocol).await;
}

async fn accept_handler(endpoint: Endpoint, source_socket: &SocketAddr, protocol: Protocol) {
    loop {
        let incoming = match endpoint.accept().await {
            Some(incoming) => incoming,
            None => {
                break;
            }
        };

        let remote_addr_log = format!("{} :: ", incoming.remote_address());

        let connection = match incoming.await {
            Ok(connection) => connection,
            Err(message) => {
                println!("{}{}", remote_addr_log.bold().red(), message);
                return;
            }
        };

        println!(
            "{}{}",
            remote_addr_log.yellow().bold(),
            "New client connected. Waiting for bidirectional negotiation..."
        );

        if let Ok(remote_nodeid) = connection.remote_node_id() {
            display_info::print(&endpoint, remote_nodeid, &remote_addr_log);
        }

        tokio::spawn(
            incoming_handle(remote_addr_log, connection, source_socket.clone(), protocol.clone())
        );
    }
}

async fn incoming_handle(
    remote_addr_log: String,
    connection: Connection,
    source_socket: SocketAddr,
    protocol: Protocol
) {
    // Accept the client's request.
    // To check the connection, we'll do ping pong.
    let instance_stream = match connection.accept_bi().await {
        Ok(instance_stream) => instance_stream,
        Err(message) => {
            println!("{}{}", remote_addr_log.bold().red(), message);
            return;
        }
    };

    match protocol {
        Protocol::Tcp =>
            tcp::connection_bridge(source_socket, remote_addr_log, instance_stream).await,
        Protocol::Udp =>
            udp::connection_bridge(source_socket, remote_addr_log, instance_stream).await,
    }
}
