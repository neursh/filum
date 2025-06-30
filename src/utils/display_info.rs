use colored::Colorize;
use iroh::{ endpoint::ConnectionType, Endpoint, PublicKey };

pub fn print(endpoint: &Endpoint, nodeid: PublicKey, addr_log: &String) {
    if let Some(info) = endpoint.remote_info(nodeid) {
        let (addr_log_colored, connection_message_colored) = match info.conn_type {
            ConnectionType::Direct(_) =>
                (addr_log.green(), "You are directly connected to the host.".green()),
            ConnectionType::Relay(relay_url) =>
                (
                    addr_log.yellow(),
                    format!("Your connection is relayed over {}", relay_url).yellow(),
                ),
            ConnectionType::Mixed(_, relay_url) =>
                (
                    addr_log.yellow(),
                    format!("Your connection is somehow directly connected AND relayed over {}", relay_url).yellow(),
                ),
            ConnectionType::None => (addr_log.red(), "How tho?".red()),
        };

        println!("{}{}", addr_log_colored, connection_message_colored);

        if let Some(latency) = info.latency {
            println!("{}Latency: {}ms", addr_log.green(), latency.as_millis());
        }
    }
}
