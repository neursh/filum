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
    let (hosting_stream, send_latency, receive_latency) = match connection.open_bi().await {
        Ok(mut hosting_stream) => {
            // Send a single bit to the peer to confirm the request.
            let send_latency = quanta::Instant::now();
            if let Err(message) = hosting_stream.0.write(&[1]).await {
                println!("{}{}", addr_log.red().bold(), message);
                return Err(());
            }
            let send_latency = send_latency.elapsed();

            // Wait for the server peer to response.
            let receive_latency = quanta::Instant::now();
            if let Err(message) = hosting_stream.1.read(&mut [0]).await {
                println!("{}{}", addr_log.red().bold(), message);
                return Err(());
            }
            let receive_latency = receive_latency.elapsed();

            (hosting_stream, send_latency, receive_latency)
        }
        Err(message) => {
            println!("{}{}", addr_log.red().bold(), message);
            return Err(());
        }
    };

    println!(
        "{}{} | {}",
        addr_log.green().bold(),
        "Connection made with a bidirectional communication protocol!".green(),
        format!(
            "(Send: {}ms Recv: {}ms)",
            send_latency.as_millis(),
            receive_latency.as_millis()
        ).bright_cyan()
    );

    Ok((endpoint, connection, hosting_stream))
}
