use std::net::{ IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4 };

use tokio::{ io::AsyncWriteExt, net::TcpSocket };

use crate::{
    structs::{ minecraft_server_info::MinecraftServerInfo, postman_server_info::PostmanServerInfo },
    utils::random_port::random_port,
};

pub struct BridgeInstance {
    assigned_socket: TcpSocket,
    assigned_port: u16,
    mc_server: MinecraftServerInfo,
    postman_info: PostmanServerInfo,
}

impl BridgeInstance {
    pub async fn new(mc_server: MinecraftServerInfo, postman_info: PostmanServerInfo) -> Self {
        let port = random_port().await.unwrap();

        let socket = TcpSocket::new_v4().unwrap();
        socket.set_reuseaddr(true).unwrap();
        socket
            .bind(std::net::SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), port)))
            .unwrap();

        BridgeInstance {
            assigned_socket: socket,
            assigned_port: port,
            mc_server,
            postman_info,
        }
    }

    pub async fn connect(self) {
        let destination = self.postman_info.host;
        let node = self.assigned_socket
            .connect(
                std::net::SocketAddr::V4(
                    SocketAddrV4::new(
                        Ipv4Addr::new(
                            destination[0],
                            destination[1],
                            destination[2],
                            destination[3]
                        ),
                        self.postman_info.port
                    )
                )
            ).await
            .unwrap();

        // self.assigned_socket
        //     .local_addr()
        //     .unwrap()
        //     .ip();

        // let message: Vec<u8> = vec![1];
        // message.extend(self.assigned_socket.local_addr().unwrap().ip().octets());
        // node.write(&message);
    }
}
