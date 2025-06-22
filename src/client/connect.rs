use std::{ net::SocketAddr, sync::Arc };

use base64::{ prelude::BASE64_STANDARD, Engine };
use colored::{ ColoredString, Colorize };
use iroh::{ endpoint::{ Connection, RecvStream, SendStream, VarInt }, Endpoint, NodeId };
use tokio::{
    io::{ AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf },
    net::{ TcpSocket, TcpStream },
};

use crate::structs::Protocol;

pub async fn establish(node: &String, port: u16, protocol: Protocol) {
    let translate: Vec<&str> = node.split(".").collect();

    if translate.len() != 2 {
        println!("{} {}", ">".red(), "Invalid node ID.".red().bold());
        return;
    }

    let nodeid_arc = Arc::new(BASE64_STANDARD.decode(translate[0]).unwrap());
    let alpn_arc = Arc::new(translate[1].as_bytes().to_owned());

    let v4_addr = format!("127.0.0.1:{}", port).parse().unwrap();
    let proxy_layer = TcpSocket::new_v4().unwrap();

    if proxy_layer.bind(v4_addr).is_err() {
        println!(
            "{} {}",
            ">".red(),
            "Can't find a suitable local port or IP to create a proxy layer!".red().bold()
        );
    }

    let proxy_listener = match proxy_layer.listen(1024) {
        Ok(listener) => listener,
        Err(message) => {
            println!(
                "{} {}\n{}",
                ">".red(),
                "Dunno what happened, error logs:".red().bold(),
                message
            );
            return;
        }
    };

    println!(
        "{} {}\n{}",
        ">".green(),
        format!("Proxy server: localhost:{}", port).bright_cyan(),
        "Checking host service...".green().bold()
    );

    loop {
        let socket = match proxy_listener.accept().await {
            Ok((socket, addr)) => {
                println!("{} {:?}", "[proxy]> New connection:".green(), addr);
                (socket, addr)
            }
            Err(message) => {
                println!("{} {:?}", "[proxy]> Connection error:".green(), message);
                continue;
            }
        };

        let (reader, writer) = tokio::io::split(socket.0);

        tokio::spawn(proxy_traffic(socket.1, reader, writer, nodeid_arc.clone(), alpn_arc.clone()));
    }
}

async fn proxy_traffic(
    addr: SocketAddr,
    reader: ReadHalf<TcpStream>,
    writer: WriteHalf<TcpStream>,
    nodeid: Arc<Vec<u8>>,
    alpn: Arc<Vec<u8>>
) {
    let hosting_node = if let Ok(endpoint) = connect_node(&addr, nodeid, alpn).await {
        endpoint
    } else {
        return;
    };

    let addr_log = format!("{} ::", addr).bright_cyan().bold();

    tokio::join!(
        proxy_reader(addr_log.clone(), hosting_node.2.0, reader),
        proxy_writer(addr_log.clone(), hosting_node.2.1, writer)
    );

    hosting_node.1.close(VarInt::from_u32(0), &[0]);
    hosting_node.0.close().await;
}

async fn proxy_reader(
    addr_log: ColoredString,
    mut hosting_writer: SendStream,
    mut reader: ReadHalf<TcpStream>
) {
    let mut buffer = [0_u8; 4096];

    loop {
        let length = match reader.read(&mut buffer).await {
            Ok(length) => {
                if length != 0 {
                    length
                } else {
                    println!("{} {}", addr_log, "Client returned a 0 message in size, aborting...");
                    break;
                }
            }
            Err(message) => {
                println!(
                    "{} {}\n{}",
                    addr_log,
                    "Something when wrong. The client might disconnected, error logs:",
                    message
                );
                break;
            }
        };

        if let Err(message) = hosting_writer.write_all(&buffer[..length]).await {
            println!(
                "{} {}\n{}",
                addr_log,
                "Can't stream the packet back to hosting node, error logs:",
                message
            );
            break;
        }
    }

    let _ = hosting_writer.finish();
}

async fn proxy_writer(
    addr_log: ColoredString,
    mut hosting_reader: RecvStream,
    mut writer: WriteHalf<TcpStream>
) {
    let mut buffer = [0_u8; 4096];

    loop {
        let length = match hosting_reader.read(&mut buffer).await {
            Ok(length) => {
                if let Some(length) = length {
                    if length != 0 {
                        length
                    } else {
                        println!(
                            "{} {}",
                            addr_log,
                            "Stream returned a 0 message in size, aborting..."
                        );
                        let _ = hosting_reader.stop(VarInt::from_u32(0));
                        break;
                    }
                } else {
                    println!("{} {}", addr_log, "Stream finished.");
                    break;
                }
            }
            Err(message) => {
                println!(
                    "{} {}\n{}",
                    addr_log,
                    "Something when wrong. The hosting node might be disconnected, error logs:",
                    message
                );
                break;
            }
        };

        if let Err(message) = writer.write_all(&buffer[..length]).await {
            println!(
                "{} {}\n{}",
                addr_log,
                "Can't stream the packets back to client, error logs:",
                message
            );
            break;
        }
    }

    let _ = writer.shutdown().await;
}

async fn connect_node(
    addr: &SocketAddr,
    nodeid: Arc<Vec<u8>>,
    alpn: Arc<Vec<u8>>
) -> Result<(Endpoint, Connection, (SendStream, RecvStream)), ()> {
    let endpoint = Endpoint::builder().discovery_n0().bind().await.unwrap();
    let addr_log = format!("{} ::", addr).bright_cyan().bold();

    if nodeid.len() != 32 {
        println!(
            "{} {}",
            ">".red(),
            "Node ID from the hosting node does not match with the standard key.".red().bold()
        );
        return Err(());
    }

    println!("{} {}", addr_log, "Connecting to the hosting node...");

    let connection = match
        endpoint.connect(NodeId::from_bytes(nodeid[..32].try_into().unwrap()).unwrap(), &alpn).await
    {
        Ok(connection) => connection,
        Err(message) => {
            println!(
                "{} {}\n{}",
                addr_log,
                "Can't connect to the hosting node, error log:".red().bold(),
                message
            );
            return Err(());
        }
    };

    println!(
        "{} {}",
        addr_log,
        "Attempting to request a bidirectional communication protocol...".yellow()
    );

    let hosting_stream = match connection.open_bi().await {
        Ok(hosting_stream) => hosting_stream,
        Err(message) => {
            println!(
                "{} {}\n{}",
                addr_log,
                "The hosting node did not accept the request, error logs:".red().bold(),
                message
            );
            return Err(());
        }
    };

    println!(
        "{} {}",
        addr_log,
        "Connection made with a bidirectional communication protocol!".green().bold()
    );

    Ok((endpoint, connection, hosting_stream))
}
