use std::net::{ Ipv4Addr, SocketAddrV4 };

use tokio::net::TcpSocket;

pub async fn random_port() -> Option<u16> {
    let dummy_socket = TcpSocket::new_v4().unwrap();
    dummy_socket.set_reuseaddr(true).unwrap();

    if
        dummy_socket
            .bind(std::net::SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 0)))
            .is_ok()
    {
        return Some(dummy_socket.local_addr().unwrap().port());
    }

    None
}
