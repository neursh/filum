use std::{ net::SocketAddr, sync::Arc };
use std::net::IpAddr;

use colored::Colorize;
use dashmap::DashMap;
use iroh::{ endpoint::{ Connection, RecvStream, SendStream }, Endpoint };
use tokio::{
    io::{ AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf },
    net::{ TcpSocket, TcpStream },
    sync::Mutex,
};

use crate::utils::compose::{ self, Signal };
use crate::utils::constants::{ ADDR_KEY_SIZE, BUFFER_SIZE, METADATA_SIZE, PORT_START };

/// Connect client to server over a ghost socket.
pub async fn connection_bridge(
    output_socket: SocketAddr,
    endpoint: (Endpoint, Connection, (SendStream, RecvStream))
) {
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
            println!("{} {}", ">".red(), message);
            return;
        }
    };

    // Structure:
    // Key: [0..16] ip address | [16..18] port
    // Value: WriteHalf of the corresponding socket
    let writers_map: Arc<DashMap<[u8; ADDR_KEY_SIZE], WriteHalf<TcpStream>>> = Arc::new(
        DashMap::new()
    );

    let hosting_writer = Arc::new(Mutex::new(endpoint.2.0));

    // Run a server_cast task to handle every sockets that are connected to the host.
    tokio::spawn(
        server_cast(addr_log.clone(), endpoint.2.1, hosting_writer.clone(), writers_map.clone())
    );

    loop {
        let socket = match proxy_listener.accept().await {
            Ok((socket, addr)) => { (socket, addr) }
            Err(message) => {
                println!("{}{}", "Connection error:".green(), message);
                continue;
            }
        };

        let (reader, writer) = tokio::io::split(socket.0);

        let mut raw_addr = match socket.1.ip() {
            IpAddr::V4(ipv4) => {
                let mut fitter = [0_u8; ADDR_KEY_SIZE];
                fitter[..4].copy_from_slice(&ipv4.to_bits().to_be_bytes());
                fitter
            }
            IpAddr::V6(ipv6) => {
                let mut fitter = [0_u8; ADDR_KEY_SIZE];
                fitter[..PORT_START].copy_from_slice(&ipv6.to_bits().to_be_bytes());
                fitter
            }
        };
        raw_addr[PORT_START..ADDR_KEY_SIZE].copy_from_slice(&socket.1.port().to_be_bytes());

        {
            writers_map.insert(raw_addr.clone(), writer);
        }

        let client_log = format!("{} :: ", socket.1);
        tokio::spawn(
            client_cast(client_log, raw_addr, reader, writers_map.clone(), hosting_writer.clone())
        );
    }
}

/// Cast server packets over instance to client.
async fn server_cast(
    addr_log: String,
    mut hosting_reader: RecvStream,
    hosting_writer: Arc<Mutex<SendStream>>,
    writers_map: Arc<DashMap<[u8; ADDR_KEY_SIZE], WriteHalf<TcpStream>>>
) {
    loop {
        // Read the metadata from server.
        let (raw_addr, packet_length, signal) = match
            compose::read_and_parse_metadata(&mut hosting_reader).await
        {
            Ok(metadata) => metadata,
            Err(message) => {
                println!("{}{}", addr_log, message);
                continue;
            }
        };

        // Check if the client is asking for disconnection.
        if signal == Signal::Dead {
            if let Some(mut writer) = writers_map.get_mut(&raw_addr) {
                let _ = writer.shutdown().await;
                drop(writer);
                writers_map.remove(&raw_addr);
            }
            continue;
        }

        // Read the actual packets from server if nothing goes wrong.
        // We'll handle when `packet_length` is 0, after sending nothing over to client.
        let mut packet = vec![0_u8; packet_length];
        if let Err(message) = hosting_reader.read_exact(&mut packet).await {
            println!("{}{}", addr_log, message);
            continue;
        }

        // Write the packets over to the actual socket on client.
        {
            let mut writer = match writers_map.get_mut(&raw_addr) {
                Some(writer) => writer,
                None => {
                    println!(
                        "{} Packet sent to a unknown client, telling host to shut it down.",
                        ">".red()
                    );
                    {
                        let mut composer = vec![0_u8; 21];
                        compose::create_message(&mut composer, &[], 0, raw_addr, Signal::Dead);
                        let _ = hosting_writer
                            .lock().await
                            .write_all(&composer[..METADATA_SIZE]).await;
                    }
                    continue;
                }
            };

            if let Err(message) = writer.write_all(&packet).await {
                println!("{}{}", addr_log, message);
                // Yej, just drop the writer lock and remove the whole thang.
                drop(writer);
                {
                    writers_map.remove(&raw_addr);
                }
                continue;
            }

            // Remove everything, we bail.
            if packet_length == 0 {
                println!("{}{}", addr_log, "Disconnected.");
                // Remove the address when there's nothing sent over (disconnected).
                let _ = writer.shutdown().await;
                drop(writer);
                {
                    writers_map.remove(&raw_addr);
                }
                continue;
            }
        }
    }
}

/// Cast client packets back to server.
async fn client_cast(
    client_log: String,
    raw_addr: [u8; ADDR_KEY_SIZE],
    mut reader: ReadHalf<TcpStream>,
    writers_map: Arc<DashMap<[u8; ADDR_KEY_SIZE], WriteHalf<TcpStream>>>,
    hosting_writer: Arc<Mutex<SendStream>>
) {
    let mut packet = [0_u8; BUFFER_SIZE];
    let mut composer = Vec::from(&raw_addr[..]);

    loop {
        // Read from client.
        // When length is 0, meaning the client is disconnected, don't break the loop just yet.
        // We'll send a final message, passing over a 0 length packet.
        let length = match reader.read(&mut packet).await {
            Ok(length) => { length }
            Err(message) => {
                println!("{}{}", client_log, message);
                break;
            }
        };

        // Pass off to create a message that can be send over Filum.
        compose::create_message(&mut composer, &packet, length, raw_addr, if length > 0 {
            Signal::Alive
        } else {
            Signal::Dead
        });

        {
            let mut writer = hosting_writer.lock().await;
            if let Err(message) = writer.write_all(&composer[..length + METADATA_SIZE]).await {
                println!("{}{}", client_log, message);
                break;
            }
        }

        // After sending an empty message to notify the server that client is gone, break this loop.
        if length == 0 {
            println!("{}{}", client_log, "Disconnected.");
            break;
        }
    }

    // Always remove the writer when out of the loop.
    {
        if let Some(mut writer) = writers_map.get_mut(&raw_addr) {
            let _ = writer.shutdown().await;
            drop(writer);
            writers_map.remove(&raw_addr);
        }
    }
}
