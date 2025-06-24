use std::net::SocketAddr;

use base64::{ prelude::BASE64_STANDARD, Engine };
use colored::Colorize;
use iroh::{ endpoint::Incoming, Endpoint };
use nanoid::nanoid;

use crate::{ host::{ tcp, udp }, structs::Protocol };

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

        tokio::spawn(incoming_handle(incoming, source_socket.clone(), protocol.clone()));
    }
}

async fn incoming_handle(incoming: Incoming, source_socket: SocketAddr, protocol: Protocol) {
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

    // Accept the client's request.
    // To check the connection, we'll do ping pong.
    let (client_stream, receive_latency, send_latency) = match connection.accept_bi().await {
        Ok(mut client_stream) => {
            // Wait for client to send back a single bit to confirm.
            let receive_latency = quanta::Instant::now();
            if let Err(message) = client_stream.1.read(&mut [1]).await {
                println!("{}{}", remote_addr_log.bold().red(), message);
                return;
            }
            let receive_latency = receive_latency.elapsed();

            // Send a confirm bit back to client.
            let send_latency = quanta::Instant::now();
            if let Err(message) = client_stream.0.write(&[1]).await {
                println!("{}{}", remote_addr_log.bold().red(), message);
                return;
            }
            let send_latency = send_latency.elapsed();

            (client_stream, receive_latency, send_latency)
        }
        Err(message) => {
            println!("{}{}", remote_addr_log.bold().red(), message);
            return;
        }
    };

    let bridge_addr: SocketAddr = match source_socket.is_ipv4() {
        true => "127.0.0.1:0".parse().unwrap(),
        false => "[::1]:0".parse().unwrap(),
    };

    println!(
        "{}{} | {}",
        remote_addr_log.bold().green(),
        "Client stream established.",
        format!(
            "(Send: {}ms Recv: {}ms)",
            send_latency.as_millis(),
            receive_latency.as_millis()
        ).bright_cyan()
    );

    match protocol {
        Protocol::Tcp =>
            tcp::connection_bridge(
                source_socket,
                bridge_addr,
                remote_addr_log,
                client_stream
            ).await,
        Protocol::Udp =>
            udp::connection_bridge(
                source_socket,
                bridge_addr,
                remote_addr_log,
                client_stream
            ).await,
    }
}
