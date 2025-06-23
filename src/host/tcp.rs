use colored::Colorize;
use iroh::endpoint::{ RecvStream, SendStream, VarInt };
use tokio::{ io::{ AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf }, net::TcpStream };

// Cast server packets over proxy to client.
pub async fn server_cast(
    mut reader: ReadHalf<TcpStream>,
    mut client_writer: SendStream,
    addr_log: String
) {
    let mut buffer = [0_u8; 4096];

    loop {
        let length = match reader.read(&mut buffer).await {
            Ok(length) => {
                if length != 0 {
                    length
                } else {
                    println!("{}{}", addr_log.bold().yellow(), "Server disconnected.");
                    break;
                }
            }
            Err(message) => {
                println!(
                    "{}{}\n{}",
                    addr_log.bold().red(),
                    "Something when wrong. Can't bridge between server and proxy, error log:",
                    message
                );
                break;
            }
        };

        if let Err(message) = client_writer.write_all(&buffer[..length]).await {
            println!(
                "{}{}\n{}",
                addr_log.bold().red(),
                "Can't stream the packet back to client proxy, error log:",
                message
            );
            break;
        }
    }

    let _ = client_writer.finish();
}

// Cast client packets over proxy to server.
pub async fn client_cast(
    mut writer: WriteHalf<TcpStream>,
    mut client_reader: RecvStream,
    addr_log: String
) {
    let mut buffer = [0_u8; 4096];

    loop {
        let length = match client_reader.read(&mut buffer).await {
            Ok(length) => {
                if let Some(length) = length {
                    if length != 0 {
                        length
                    } else {
                        println!(
                            "{}{}",
                            addr_log.bold().red(),
                            "Stream returned a 0 message in size, aborting..."
                        );
                        let _ = client_reader.stop(VarInt::from_u32(0));
                        break;
                    }
                } else {
                    println!("{}{}", addr_log.bold().yellow(), "Stream finished.");
                    break;
                }
            }
            Err(message) => {
                println!(
                    "{}{}\n{}",
                    addr_log.bold().red(),
                    "Something when wrong. The client node might be disconnected, error logs:",
                    message
                );
                break;
            }
        };

        if let Err(message) = writer.write_all(&buffer[..length]).await {
            println!(
                "{}{}\n{}",
                addr_log.bold().red(),
                "Can't stream the packets back to server, error logs:",
                message
            );
            break;
        }
    }
}
