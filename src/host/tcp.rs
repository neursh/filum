use std::net::SocketAddr;

use colored::Colorize;
use iroh::endpoint::{ RecvStream, SendStream, VarInt };
use tokio::{
    io::{ AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf },
    net::{ TcpSocket, TcpStream },
};

/// Connect client to server over a ghost socket.
pub async fn connection_bridge(
    source_socket: SocketAddr,
    remote_addr_log: String,
    mut client_stream: (SendStream, RecvStream)
) {
    let bridge_addr: SocketAddr = match source_socket.is_ipv4() {
        true => "127.0.0.1:0".parse().unwrap(),
        false => "[::1]:0".parse().unwrap(),
    };
    
    let proxied_server = match source_socket.is_ipv4() {
        true => TcpSocket::new_v4().unwrap(),
        false => TcpSocket::new_v6().unwrap(),
    };

    if proxied_server.bind(bridge_addr).is_err() {
        println!(
            "{}{}",
            remote_addr_log.bold().red(),
            "Can't find a suitable port or IP to create a proxy layer over to client. Aborting..."
        );
        let _ = client_stream.1.stop(VarInt::from_u32(0));
        let _ = client_stream.0.finish();
        return;
    }

    let proxied_stream = match proxied_server.connect(source_socket).await {
        Ok(proxied_stream) => proxied_stream,
        Err(message) => {
            println!("{}{}", remote_addr_log.bold().red(), message);
            let _ = client_stream.1.stop(VarInt::from_u32(0));
            let _ = client_stream.0.finish();
            return;
        }
    };

    let (reader, writer) = tokio::io::split(proxied_stream);

    tokio::join!(
        server_cast(reader, client_stream.0, remote_addr_log.clone()),
        client_cast(writer, client_stream.1, remote_addr_log.clone())
    );
}

/// Cast server packets over proxy to client.
async fn server_cast(
    mut reader: ReadHalf<TcpStream>,
    mut client_writer: SendStream,
    remote_addr_log: String
) {
    let mut buffer = [0_u8; 4096];

    loop {
        let length = match reader.read(&mut buffer).await {
            Ok(length) => {
                if length == 0 {
                    println!("{}{}", remote_addr_log.bold().yellow(), "Server disconnected.");
                    break;
                }
                length
            }
            Err(message) => {
                println!("{}{}", remote_addr_log.bold().red(), message);
                break;
            }
        };

        if let Err(message) = client_writer.write_all(&buffer[..length]).await {
            println!("{}{}", remote_addr_log.bold().red(), message);
            break;
        }
    }

    // Finally, clear everything.
    let _ = client_writer.finish();
}

/// Cast client packets over proxy to server.
async fn client_cast(
    mut writer: WriteHalf<TcpStream>,
    mut client_reader: RecvStream,
    remote_addr_log: String
) {
    let mut buffer = [0_u8; 4096];

    loop {
        let length = match client_reader.read(&mut buffer).await {
            Ok(length) => {
                if let Some(length) = length {
                    if length == 0 {
                        println!("{}{}", remote_addr_log.bold().yellow(), "Client disconnected.");
                        break;
                    }
                    length
                } else {
                    println!("{}{}", remote_addr_log.bold().yellow(), "Client disconnected.");
                    break;
                }
            }
            Err(message) => {
                println!("{}{}", remote_addr_log.bold().red(), message);
                break;
            }
        };

        if let Err(message) = writer.write_all(&buffer[..length]).await {
            println!("{}{}", remote_addr_log.bold().red(), message);
            break;
        }
    }

    // Finally, clear everything.
    let _ = client_reader.stop(VarInt::from_u32(0));
    let _ = writer.shutdown().await;
}
