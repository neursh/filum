use std::{ net::SocketAddr, sync::Arc };

use colored::Colorize;
use iroh::endpoint::{ RecvStream, SendStream, VarInt };
use tokio::{ net::UdpSocket, sync::RwLock };

/// Connect client to server over a ghost socket.
pub async fn connection_bridge(
    source_socket: SocketAddr,
    bridge_addr: SocketAddr,
    remote_addr_log: String,
    mut client_stream: (SendStream, RecvStream)
) {
    let proxied_server = match UdpSocket::bind(bridge_addr).await {
        Ok(socket) => socket,
        Err(_) => {
            println!(
                "{}{}",
                remote_addr_log.bold().red(),
                "Can't find a suitable port or IP to create a proxy layer over to client."
            );
            let _ = client_stream.1.stop(VarInt::from_u32(0));
            let _ = client_stream.0.finish();
            return;
        }
    };

    match proxied_server.connect(source_socket).await {
        Ok(proxy_stream) => proxy_stream,
        Err(message) => {
            println!("{}{}", remote_addr_log.bold().red(), message);
            let _ = client_stream.1.stop(VarInt::from_u32(0));
            let _ = client_stream.0.finish();
            return;
        }
    }

    // Wrap `proxied_server` inside Arc and RwLock to pass over 2 tasks.
    let proxied_server = Arc::new(RwLock::new(proxied_server));

    tokio::join!(
        server_cast(proxied_server.clone(), client_stream.0, remote_addr_log.clone()),
        client_cast(proxied_server.clone(), client_stream.1, remote_addr_log.clone())
    );
}

/// Cast server packets over proxy to client.
async fn server_cast(
    proxied_server: Arc<RwLock<UdpSocket>>,
    mut client_writer: SendStream,
    addr_log: String
) {
    let mut buffer = [0_u8; 4096];

    let proxied_server_read = proxied_server.read().await;
    loop {
        let length = match proxied_server_read.recv(&mut buffer).await {
            Ok(length) => {
                if length == 0 {
                    println!("{}{}", addr_log.bold().yellow(), "Server disconnected.");
                    break;
                }
                length
            }
            Err(message) => {
                println!("{}{}", addr_log.bold().red(), message);
                break;
            }
        };

        if let Err(message) = client_writer.write_all(&buffer[..length]).await {
            println!("{}{}", addr_log.bold().red(), message);
            break;
        }
    }

    // Finally, clear everything.
    let _ = client_writer.finish();
}

/// Cast client packets over proxy to server.
async fn client_cast(
    proxied_server: Arc<RwLock<UdpSocket>>,
    mut client_reader: RecvStream,
    addr_log: String
) {
    let mut buffer = [0_u8; 4096];

    let proxied_server_write = proxied_server.read().await;
    loop {
        let length = match client_reader.read(&mut buffer).await {
            Ok(length) => {
                if let Some(length) = length {
                    if length == 0 {
                        println!("{}{}", addr_log.bold().yellow(), "Client disconnected.");
                        break;
                    }
                    length
                } else {
                    println!("{}{}", addr_log.bold().yellow(), "Stream finished.");
                    break;
                }
            }
            Err(message) => {
                println!("{}{}", addr_log.bold().red(), message);
                break;
            }
        };

        if let Err(message) = proxied_server_write.send(&buffer[..length]).await {
            println!("{}{}", addr_log.bold().red(), message);
            break;
        }
    }

    // Finally, clear everything.
    let _ = client_reader.stop(VarInt::from_u32(0));
}
