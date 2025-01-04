mod bridge;
mod postman;
mod client;
pub mod structs;
pub mod utils;

use bridge::instance::BridgeInstance;
use structs::{ minecraft_server_info::MinecraftServerInfo, postman_server_info::PostmanServerInfo };

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

    PostmanInstance::new(23634).start().await;
}
