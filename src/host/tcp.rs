use std::{ net::SocketAddr, sync::Arc };

use colored::Colorize;
use dashmap::DashMap;
use iroh::endpoint::{ RecvStream, SendStream };
use tokio::{
    io::{ AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf },
    net::{ TcpSocket, TcpStream },
    sync::{ mpsc::{ self, Receiver, Sender }, Mutex },
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
    let clients_map: Arc<DashMap<[u8; 18], Sender<Vec<u8>>>> = Arc::new(DashMap::new());

    let endpoint_writer = Arc::new(Mutex::new(instance_stream.0));

    loop {
        let mut metadata = [0_u8; 20];
        if let Err(message) = instance_stream.1.read_exact(&mut metadata).await {
            println!("{}{}", remote_addr_log, message);
            continue;
        }

        // Parse the metadata
        let raw_addr: [u8; 18] = metadata[0..18].try_into().unwrap();
        let packet_length = u16::from_be_bytes(metadata[18..20].try_into().unwrap()) as usize;

        // Read the actual packets from server if nothing goes wrong.
        let mut packet = vec![0_u8; packet_length];
        if let Err(message) = instance_stream.1.read_exact(&mut packet).await {
            println!("{}{}", remote_addr_log, message);
            continue;
        }

        {
            let writer = clients_map.get(&raw_addr);
            match writer {
                Some(packet_sender) => {
                    if packet_sender.send(packet).await.is_err() {
                        drop(packet_sender);
                        {
                            clients_map.remove(&raw_addr);
                        }
                    }
                }
                None => {
                    let (packet_sender, packet_receiver): (
                        Sender<Vec<u8>>,
                        Receiver<Vec<u8>>,
                    ) = mpsc::channel(4069);
                    let _ = packet_sender.send(packet).await;
                    {
                        clients_map.insert(raw_addr, packet_sender);
                    }
                    tokio::spawn(
                        make_new_socket(
                            remote_addr_log.clone(),
                            source_socket.clone(),
                            packet_receiver,
                            endpoint_writer.clone(),
                            raw_addr,
                            clients_map.clone()
                        )
                    );
                }
            }
        }
    }
}

async fn make_new_socket(
    remote_addr_log: String,
    source_socket: SocketAddr,
    mut packet_receiver: Receiver<Vec<u8>>,
    endpoint_writer: Arc<Mutex<SendStream>>,
    raw_addr: [u8; 18],
    clients_map: Arc<DashMap<[u8; 18], Sender<Vec<u8>>>>
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
            remote_addr_log.bold().red(),
            "Can't find a suitable port or IP to create a proxy layer over to client. Aborting..."
        );
        packet_receiver.close();
        return;
    }

    let proxied_stream = match proxied_client.connect(source_socket).await {
        Ok(proxied_stream) => proxied_stream,
        Err(message) => {
            println!("{}{}", remote_addr_log.bold().red(), message);
            packet_receiver.close();
            return;
        }
    };

    let (reader, writer) = tokio::io::split(proxied_stream);

    tokio::join!(
        server_cast(
            remote_addr_log.clone(),
            raw_addr,
            reader,
            endpoint_writer.clone(),
            clients_map.clone()
        ),
        client_cast(remote_addr_log, raw_addr, writer, packet_receiver, clients_map)
    );
}

/// Cast server packets over proxy to client.
async fn server_cast(
    remote_addr_log: String,
    raw_addr: [u8; 18],
    mut reader: ReadHalf<TcpStream>,
    endpoint_writer: Arc<Mutex<SendStream>>,
    clients_map: Arc<DashMap<[u8; 18], Sender<Vec<u8>>>>
) {
    let mut packet = [0_u8; 4096];
    let mut composer = raw_addr.to_vec();

    loop {
        let length = match reader.read(&mut packet).await {
            Ok(length) => {
                if length == 0 {
                    println!("{}{}", remote_addr_log.bold().yellow(), "Disconnected.");
                    break;
                }
                length
            }
            Err(message) => {
                println!("{}{}", remote_addr_log.bold().red(), message);
                break;
            }
        };

        {
            composer.resize(length + 20, 0);
            composer[18..20].copy_from_slice(&(length as u16).to_be_bytes());
            composer[20..length + 20].copy_from_slice(&packet[..length]);
            if
                let Err(message) = endpoint_writer
                    .lock().await
                    .write_all(&composer[0..length + 20]).await
            {
                println!("{}{}", remote_addr_log.bold().red(), message);
                break;
            }
        }
    }

    clients_map.remove(&raw_addr);
}

/// Cast client packets over proxy to server.
async fn client_cast(
    remote_addr_log: String,
    raw_addr: [u8; 18],
    mut writer: WriteHalf<TcpStream>,
    mut packet_receiver: Receiver<Vec<u8>>,
    clients_map: Arc<DashMap<[u8; 18], Sender<Vec<u8>>>>
) {
    loop {
        let packet = match packet_receiver.recv().await {
            Some(packet) => packet,
            None => {
                packet_receiver.close();
                return;
            }
        };

        if let Err(message) = writer.write_all(&packet).await {
            println!("{}{}", remote_addr_log.bold().red(), message);
            break;
        }
    }

    // Finally, clear everything.
    let _ = writer.shutdown().await;
    clients_map.remove(&raw_addr);
}
