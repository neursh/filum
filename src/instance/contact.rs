use colored::Colorize;
use iroh::{ endpoint::{ Connection, RecvStream, SendStream }, Endpoint, NodeId };

pub async fn endpoint(
    addr_log: String,
    nodeid: [u8; 32],
    alpn: Vec<u8>
) -> Result<(Endpoint, Connection, (SendStream, RecvStream)), ()> {
    let endpoint = Endpoint::builder().discovery_n0().bind().await.unwrap();

    if nodeid.len() != 32 {
        println!(
            "{} {}",
            ">".red(),
            "Node ID from the hosting node does not match with the standard key.".red().bold()
        );
        return Err(());
    }

    println!("{}{}", addr_log.yellow().bold(), "Connecting to the hosting node...");

    let connection = match endpoint.connect(NodeId::from_bytes(&nodeid).unwrap(), &alpn).await {
        Ok(connection) => connection,
        Err(message) => {
            println!("{}{}", addr_log.red().bold(), message);
            return Err(());
        }
    };

    println!(
        "{}{}",
        addr_log.yellow().bold(),
        "Attempting to request a bidirectional communication protocol...".yellow()
    );

    // Request host to accept both ways communication.
    // To check the connection, we'll do ping pong.
    let hosting_stream = match connection.open_bi().await {
        Ok(hosting_stream) => hosting_stream,
        Err(message) => {
            println!("{}{}", addr_log.red().bold(), message);
            return Err(());
        }
    };

    Ok((endpoint, connection, hosting_stream))
}
