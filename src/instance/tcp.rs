use std::net::SocketAddr;

use colored::Colorize;
use iroh::endpoint::{ RecvStream, SendStream, VarInt };
use tokio::{
    io::{ AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf },
    net::{ TcpSocket, TcpStream },
};

use crate::instance::contact;

/// Connect client to server over a ghost socket.
pub async fn connection_bridge(output_socket: SocketAddr, nodeid: [u8; 32], alpn: Vec<u8>) {
    let addr_log = format!("{} :: ", output_socket);

    let proxy_layer = (
        match output_socket.is_ipv4() {
            true => TcpSocket::new_v4(),
            false => TcpSocket::new_v6(),
        }
    ).unwrap();

    if let Err(message) = proxy_layer.bind(output_socket) {
        println!("{} {}", ">".red(), message);
    }

    let proxy_listener = match proxy_layer.listen(1024) {
        Ok(listener) => listener,
        Err(message) => {
            println!("{}{}", ">".red(), message);
            return;
        }
    };

    loop {
        let socket = match proxy_listener.accept().await {
            Ok((socket, addr)) => {
                println!("{}{}", "New connection:".green(), addr);
                (socket, addr)
            }
            Err(message) => {
                println!("{}{}", "Connection error:".green(), message);
                continue;
            }
        };

        let (reader, writer) = tokio::io::split(socket.0);

        tokio::spawn(proxy_traffic(addr_log.clone(), reader, writer, nodeid.clone(), alpn.clone()));
    }
}

async fn proxy_traffic(
    addr_log: String,
    reader: ReadHalf<TcpStream>,
    writer: WriteHalf<TcpStream>,
    nodeid: [u8; 32],
    alpn: Vec<u8>
) {
    let hosting_node = if
        let Ok(endpoint) = contact::endpoint(addr_log.clone(), nodeid, alpn).await
    {
        endpoint
    } else {
        return;
    };

    tokio::join!(
        server_cast(addr_log.clone(), hosting_node.2.1, writer),
        client_cast(addr_log.clone(), hosting_node.2.0, reader)
    );

    hosting_node.1.close(VarInt::from_u32(0), &[0]);
    hosting_node.0.close().await;
}

/// Cast server packets over proxy to client.
async fn server_cast(
    addr_log: String,
    mut hosting_reader: RecvStream,
    mut writer: WriteHalf<TcpStream>
) {
    let mut buffer = [0_u8; 4096];

    loop {
        let length = match hosting_reader.read(&mut buffer).await {
            Ok(length) => {
                if let Some(length) = length {
                    if length == 0 {
                        println!("{}{}", addr_log, "Server disconnected.");
                        let _ = hosting_reader.stop(VarInt::from_u32(0));
                        break;
                    }
                    length
                } else {
                    println!("{}{}", addr_log, "Stream finished.");
                    break;
                }
            }
            Err(message) => {
                println!("{}{}", addr_log, message);
                break;
            }
        };

        if let Err(message) = writer.write_all(&buffer[..length]).await {
            println!("{}{}", addr_log, message);
            break;
        }
    }

    let _ = writer.shutdown().await;
}

/// Cast client packets back to server.
async fn client_cast(
    addr_log: String,
    mut hosting_writer: SendStream,
    mut reader: ReadHalf<TcpStream>
) {
    let mut buffer = [0_u8; 4096];

    loop {
        let length = match reader.read(&mut buffer).await {
            Ok(length) => {
                if length == 0 {
                    println!("{}{}", addr_log, "Client disconnected.");
                    break;
                }
                length
            }
            Err(message) => {
                println!("{}{}", addr_log, message);
                break;
            }
        };

        if let Err(message) = hosting_writer.write_all(&buffer[..length]).await {
            println!("{}{}", addr_log, message);
            break;
        }
    }

    let _ = hosting_writer.finish();
}
