mod bridge;
mod client;
pub mod structs;
pub mod utils;

use std::net::{ IpAddr, Ipv4Addr, SocketAddr, ToSocketAddrs };

use bridge::instance::BridgeInstance;
use structs::{ minecraft_server_info::MinecraftServerInfo, postman_server_info::PostmanServerInfo };
use tokio::{ io::{ AsyncReadExt, AsyncWriteExt }, net::TcpSocket };
use rand::RngCore;

#[tokio::main]
async fn main() {
    let mc_server = MinecraftServerInfo {
        host: [127, 0, 0, 1],
        port: 25565,
    };
    let postman_info = PostmanServerInfo {
        host: [127, 0, 0, 1],
        port: 63841,
    };

    let socket = TcpSocket::new_v4().unwrap();
    socket.set_reuseaddr(true).unwrap();
    socket.bind("0.0.0.0:26241".parse().unwrap()).unwrap();

    let mut client = socket
        .connect(
            "stunserver2024.stunprotocol.org:3478".to_socket_addrs().unwrap().next().unwrap()
        ).await
        .unwrap();

    let request_header = [0u8, 1, 0, 0];
    let cookie = [33u8, 18, 164, 66];
    let mut transaction_id = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut transaction_id);

    let mut message = [0u8; 20];
    message[0..4].copy_from_slice(&request_header);
    message[4..8].copy_from_slice(&cookie);
    message[8..20].copy_from_slice(&transaction_id);

    client.write(&message).await.unwrap();

    let mut buf: [u8; 100] = [0; 100];
    let length = client.read(&mut buf).await.unwrap();

    println!("{}", length);

    println!("{:?}", buf);
}
