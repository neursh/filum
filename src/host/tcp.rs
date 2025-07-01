use std::{ net::SocketAddr, sync::Arc };

use colored::Colorize;
use dashmap::DashMap;
use iroh::endpoint::{ RecvStream, SendStream };
use tokio::{
    io::{ AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf },
    net::{ TcpSocket, TcpStream },
    select,
    sync::{ mpsc::{ self, Receiver, Sender }, Mutex },
};
use tokio_util::sync::CancellationToken;

use crate::utils::{
    compose::{ self, Signal },
    constants::{ ADDR_KEY_SIZE, BUFFER_SIZE, CHANNEL_SIZE, METADATA_SIZE, PORT_START },
};

/// Connect client to server over a ghost socket.
pub async fn connection_bridge(
    source_socket: SocketAddr,
    remote_addr_log: String,
    mut instance_stream: (SendStream, RecvStream)
) {
    // Structure:
    // Key: [0..16] ip address | [16..18] port
    // Value: Sender for packets read by Endpoint to pass over to the handling task.
    let clients_map: Arc<
        DashMap<[u8; ADDR_KEY_SIZE], (Sender<Vec<u8>>, CancellationToken)>
    > = Arc::new(DashMap::new());

    let endpoint_writer = Arc::new(Mutex::new(instance_stream.0));

    loop {
        // Read the metadata.
        let (raw_addr, packet_length, signal) = match
            compose::read_and_parse_metadata(&mut instance_stream.1).await
        {
            Ok(metadata) => metadata,
            Err(message) => {
                println!("{}{}", format!("{}-> Filum :: ", remote_addr_log).red(), message);
                // We'll break and remove the Endpoint.
                break;
            }
        };

        // Check if the client is asking for disconnection.
        if signal == Signal::Dead {
            client_clean_up(clients_map.clone(), raw_addr).await;
            continue;
        }

        // Read the actual packets from client if nothing goes wrong.
        let mut packet = vec![0_u8; packet_length];
        if let Err(message) = instance_stream.1.read_exact(&mut packet).await {
            println!("{}{}", format!("{}-> Filum :: ", remote_addr_log).red(), message);
            // We'll break and remove the Endpoint.
            client_clean_up(clients_map.clone(), raw_addr).await;
            break;
        }

        let client = clients_map.get(&raw_addr);
        match client {
            // First scenario: There is a client connected through Filum.
            // Send straight to message channel to be processed by the task responsible for the client.
            // If something when wrong, go ahead and call everything up.
            Some(client) => {
                if client.0.send(packet).await.is_err() {
                    // Release lock
                    drop(client);
                    client_clean_up(clients_map.clone(), raw_addr).await;
                }
            }

            // Second scenario: No socket associated with the client, create a new one.
            None => {
                let (packet_sender, packet_receiver): (
                    Sender<Vec<u8>>,
                    Receiver<Vec<u8>>,
                ) = mpsc::channel(CHANNEL_SIZE);
                // Create a token to call reader to cancel reading when client disconnects.
                let shutdown_token = CancellationToken::new();

                // Send a message buffer in queue to let the task read later.
                let _ = packet_sender.send(packet).await;

                clients_map.insert(raw_addr, (packet_sender, shutdown_token.clone()));

                let client_port_log = format!(
                    "{}-> {} :: ",
                    remote_addr_log,
                    u16::from_be_bytes(raw_addr[PORT_START..ADDR_KEY_SIZE].try_into().unwrap())
                );

                tokio::spawn(
                    make_new_socket(
                        client_port_log,
                        source_socket.clone(),
                        packet_receiver,
                        endpoint_writer.clone(),
                        raw_addr,
                        clients_map.clone(),
                        shutdown_token
                    )
                );
            }
        }
    }

    println!("{}{}", remote_addr_log.red(), "Instance disconnected.".red());

    // Clean up the bridge when instance `Endpoint` left.
    for client in clients_map.iter() {
        client.1.cancel();
    }
}

async fn make_new_socket(
    client_port_log: String,
    source_socket: SocketAddr,
    mut packet_receiver: Receiver<Vec<u8>>,
    endpoint_writer: Arc<Mutex<SendStream>>,
    raw_addr: [u8; ADDR_KEY_SIZE],
    clients_map: Arc<DashMap<[u8; ADDR_KEY_SIZE], (Sender<Vec<u8>>, CancellationToken)>>,
    shutdown_token: CancellationToken
) {
    let bridge_addr: SocketAddr = match source_socket.is_ipv4() {
        true => "127.0.0.1:0".parse().unwrap(),
        false => "[::1]:0".parse().unwrap(),
    };

    let proxied_client = match source_socket.is_ipv4() {
        true => TcpSocket::new_v4().unwrap(),
        false => TcpSocket::new_v6().unwrap(),
    };

    if proxied_client.bind(bridge_addr).is_err() {
        println!(
            "{}{}",
            client_port_log.bold().red(),
            "Can't find a suitable port or IP to create a proxy layer over to client. Aborting..."
        );
        packet_receiver.close();
        return;
    }

    let proxied_stream = match proxied_client.connect(source_socket).await {
        Ok(proxied_stream) => proxied_stream,
        Err(message) => {
            println!("{}{}", client_port_log.bold().red(), message);
            packet_receiver.close();
            return;
        }
    };

    let (reader, writer) = tokio::io::split(proxied_stream);

    println!("{}Connected!", client_port_log.green());

    tokio::join!(
        server_cast(
            client_port_log.clone(),
            raw_addr,
            reader,
            endpoint_writer.clone(),
            clients_map.clone(),
            shutdown_token.clone()
        ),
        client_cast(client_port_log, raw_addr, writer, packet_receiver, clients_map, shutdown_token)
    );
}

/// Cast server packets over proxy to client.
async fn server_cast(
    client_port_log: String,
    raw_addr: [u8; ADDR_KEY_SIZE],
    mut reader: ReadHalf<TcpStream>,
    endpoint_writer: Arc<Mutex<SendStream>>,
    clients_map: Arc<DashMap<[u8; ADDR_KEY_SIZE], (Sender<Vec<u8>>, CancellationToken)>>,
    shutdown_token: CancellationToken
) {
    let mut packet = [0_u8; BUFFER_SIZE];
    let mut composer = raw_addr.to_vec();

    loop {
        // Read from server.
        // When length is 0, meaning the client is disconnected, don't break the loop just yet.
        // We'll send a disconnect signal when the length is 0.
        let packet_length;
        tokio::select! {
            // We want to poll for the shutdown_token first to avoid too much unnecessary read results when client is already gone.
            biased;

            _ = shutdown_token.cancelled() => {
                break;
            }

            length = reader.read(&mut packet) => {
                match length {
                    Ok(length) => packet_length = length,
                    Err(message) => {
                        println!("{}{}", client_port_log.bold().red(), message);
                        break;
                    }
                }
            }
        }

        // Pass off to create a message that can be send over Filum.
        compose::create_message(&mut composer, &packet, packet_length, raw_addr, if
            packet_length > 0
        {
            Signal::Alive
        } else {
            Signal::Dead
        });

        if
            let Err(message) = endpoint_writer
                .lock().await
                .write_all(&composer[..packet_length + METADATA_SIZE]).await
        {
            println!("{}{}", client_port_log.bold().red(), message);
            break;
        }

        // After sending a disconnection message to notify the client that the head is gone, break this loop.
        if packet_length == 0 {
            println!("{}Disconnected.", client_port_log.bold().yellow());
            break;
        }
    }

    // Finally, clear everything.
    client_clean_up(clients_map, raw_addr).await;
}

/// Cast client packets over proxy to server.
async fn client_cast(
    client_addr_log: String,
    raw_addr: [u8; ADDR_KEY_SIZE],
    mut writer: WriteHalf<TcpStream>,
    mut packet_receiver: Receiver<Vec<u8>>,
    clients_map: Arc<DashMap<[u8; ADDR_KEY_SIZE], (Sender<Vec<u8>>, CancellationToken)>>,
    shutdown_token: CancellationToken
) {
    loop {
        let received_packet: Vec<u8>;
        select! {
            biased;
            
            _ = shutdown_token.cancelled() => {
                break;
            }

            packet = packet_receiver.recv() => {
                match packet {
                    Some(packet) => received_packet = packet,
                    None => {
                        break;
                    }
                }
            }
        }

        if let Err(message) = writer.write_all(&received_packet).await {
            println!("{}{}", client_addr_log.bold().red(), message);
            break;
        }

        if received_packet.len() == 0 {
            break;
        }
    }

    // Finally, clear everything.
    // I guess.
    packet_receiver.close();
    let _ = writer.shutdown().await;
    client_clean_up(clients_map, raw_addr).await;
}

async fn client_clean_up(
    clients_map: Arc<DashMap<[u8; ADDR_KEY_SIZE], (Sender<Vec<u8>>, CancellationToken)>>,
    raw_addr: [u8; ADDR_KEY_SIZE]
) {
    if let Some(client) = clients_map.get(&raw_addr) {
        // Telling reader to go fuck itself.
        client.1.cancel();
        drop(client);
        clients_map.remove(&raw_addr);
    }
}
