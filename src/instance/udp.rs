use std::{ net::SocketAddr, sync::Arc, time::Duration };

use colored::Colorize;
use iroh::{endpoint::{ Connection, RecvStream, SendStream, VarInt }, Endpoint};
use moka::future::Cache;
use tokio::{ net::UdpSocket, sync::{ mpsc::{ self, Receiver }, RwLock } };

use crate::instance::contact;

/// For UDP, filum will stream everything to server.
/// This happens over a single iroh endpoint for each filum instance on server.
///
pub async fn connection_bridge(output_socket: SocketAddr, endpoint: (Endpoint, Connection, (SendStream, RecvStream))) {
    let addr_log = format!("{} :: ", output_socket);
    let proxy_layer = Arc::new(UdpSocket::bind(output_socket).await.unwrap());

    // Only one Endpoint.
    let hosting_node = if
        let Ok(endpoint) = contact::endpoint(addr_log.clone(), nodeid, alpn).await
    {
        endpoint
    } else {
        return;
    };

    let message_pass = mpsc::channel<>(4096);



    tokio::join!(
        server_cast(addr_log.clone(), hosting_node.2.1, proxy_layer),
        client_cast(addr_log.clone(), receiver, hosting_node.2.0)
    );

    hosting_node.1.close(VarInt::from_u32(0), &[0]);
    hosting_node.0.close().await;

    let mut buffer = [0_u8; 4096];
    loop {
        let info = match proxy_layer.recv_from(&mut buffer).await {
            Ok((length, addr)) => { (length, addr) }
            Err(message) => {
                println!("{}{:?}", format!("{} :: ", info).red().bold(), message);
                continue;
            }
        };
    }
}

/// Cast server packets over proxy to client.
async fn server_cast(
    addr_log: String,
    mut hosting_reader: RecvStream,
    proxy_layer: Arc<UdpSocket>
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

        println!("Server: {:?}", buffer[..length].to_owned());

        if let Err(message) = proxy_layer.send_to(&buffer[..length], client_address).await {
            println!("{}{}", addr_log, message);
            break;
        }
    }
}

/// Cast client packets back to server.
async fn client_cast(
    addr_log: String,
    mut receiver: Receiver<Vec<u8>>,
    mut hosting_writer: SendStream
) {
    loop {
        match receiver.recv().await {
            Some(buffer) => {
                if buffer.len() == 0 {
                    println!("{}{}", addr_log, "Client disconnected.");
                    break;
                }
                if let Err(message) = hosting_writer.write_all(&buffer).await {
                    println!("{}{}", addr_log, message);
                    break;
                }
            }
            None => {
                println!("{}{}", addr_log, "Client disconnected.");
                break;
            }
        }
    }

    receiver.close();
    let _ = hosting_writer.finish();
}
