use std::{ net::SocketAddr, sync::Arc };

use base64::{ prelude::BASE64_STANDARD, Engine };
use colored::Colorize;
use iroh::{ endpoint::Incoming, Endpoint };
use nanoid::nanoid;
use tokio::{ net::{ TcpSocket, UdpSocket }, sync::RwLock };

use crate::{ host::{ tcp, udp }, structs::Protocol };

pub async fn create_host(source: &str, protocol: Protocol) {
    let alpn = nanoid!(32);
    let endpoint = Arc::new(
        RwLock::new(
            Endpoint::builder()
                .discovery_n0()
                .alpns(vec![alpn.as_bytes().to_vec()])
                .bind().await
                .unwrap()
        )
    );
    let nodeid = BASE64_STANDARD.encode(endpoint.read().await.node_id().as_bytes());
    let source_socket: SocketAddr = source.parse().unwrap();

    println!(
        "{} Service started, you can now share this ID to client to let them connect to {}.",
        ">".green(),
        format!("{}", source).bright_cyan()
    );
    println!("ID: {}", format!("{}.{}", nodeid, alpn).bright_cyan());

    internal_handler(endpoint.clone(), &source_socket, protocol).await;
}

async fn internal_handler(
    endpoint: Arc<RwLock<Endpoint>>,
    source_socket: &SocketAddr,
    protocol: Protocol
) {
    let endpoint_read = endpoint.read().await;

    loop {
        let incoming = match endpoint_read.accept().await {
            Some(incoming) => incoming,
            None => {
                break;
            }
        };

        tokio::spawn(incoming_handle(incoming, source_socket.clone(), protocol.clone()));
    }
}

async fn incoming_handle(incoming: Incoming, source_socket: SocketAddr, protocol: Protocol) {
    let addr_log = format!("{} :: ", incoming.remote_address());

    let connection = match incoming.await {
        Ok(connection) => connection,
        Err(message) => {
            println!("{}{}", addr_log.bold().red(), message);
            return;
        }
    };

    let client_stream = match connection.accept_bi().await {
        Ok(client_stream) => client_stream,
        Err(message) => {
            println!("{}{}", addr_log.bold().red(), message);
            return;
        }
    };

    println!("{}{}", addr_log.bold().green(), "Client connected.");

    match protocol {
        Protocol::Tcp => {
            let (proxied_server, addr) = match source_socket.is_ipv4() {
                true => (TcpSocket::new_v4().unwrap(), "127.0.0.1:0".parse().unwrap()),
                false => (TcpSocket::new_v6().unwrap(), "[::1]:0".parse().unwrap()),
            };

            if proxied_server.bind(addr).is_err() {
                println!(
                    "{}{}",
                    addr_log.bold().red(),
                    "Can't find a suitable port or IP to create a proxy layer over to client."
                        .red()
                        .bold()
                );
            }

            let proxied_stream = match proxied_server.connect(source_socket).await {
                Ok(proxied_stream) => proxied_stream,
                Err(message) => {
                    println!("{}{}", addr_log.bold().red(), message);
                    return;
                }
            };

            let (reader, writer) = tokio::io::split(proxied_stream);

            tokio::join!(
                tcp::server_cast(reader, client_stream.0, addr_log.clone()),
                tcp::client_cast(writer, client_stream.1, addr_log.clone())
            );
        }
        Protocol::Udp => {
            let addr: SocketAddr = match source_socket.is_ipv4() {
                true => "127.0.0.1:0".parse().unwrap(),
                false => "[::1]:0".parse().unwrap(),
            };
            let proxied_server = match UdpSocket::bind(addr).await {
                Ok(socket) => {
                    match socket.connect(source_socket).await {
                        Ok(proxy_stream) => proxy_stream,
                        Err(message) => {
                            println!("{}{}", addr_log.bold().red(), message);
                            return;
                        }
                    }
                    Arc::new(RwLock::new(socket))
                }
                Err(_) => {
                    println!(
                        "{}{}",
                        addr_log.bold().red(),
                        "Can't find a suitable port or IP to create a proxy layer over to client."
                            .red()
                            .bold()
                    );
                    return;
                }
            };

            tokio::join!(
                udp::server_cast(proxied_server.clone(), client_stream.0, addr_log.clone()),
                udp::client_cast(proxied_server.clone(), client_stream.1, addr_log.clone())
            );
        }
    }
}
