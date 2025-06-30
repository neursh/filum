use iroh::endpoint::{ ReadExactError, RecvStream };

use crate::utils::constants::{ ADDR_KEY_SIZE, METADATA_SIZE };

#[derive(PartialEq)]
pub enum Signal {
    Dead,
    Alive,
}

pub fn create_message(
    result: &mut Vec<u8>,
    packet: &[u8],
    length: usize,
    raw_addr_port: [u8; ADDR_KEY_SIZE],
    signal: Signal
) {
    result.resize(length + METADATA_SIZE, 0);
    result[..ADDR_KEY_SIZE].copy_from_slice(&raw_addr_port);
    result[ADDR_KEY_SIZE..20].copy_from_slice(&(length as u16).to_be_bytes());

    match signal {
        Signal::Alive => {
            result[20] = 1;
        }
        Signal::Dead => {
            result[20] = 0;
        }
    }

    result[METADATA_SIZE..length + METADATA_SIZE].copy_from_slice(&packet[..length]);
}

pub fn parse_metadata(metadata: &[u8; METADATA_SIZE]) -> ([u8; ADDR_KEY_SIZE], usize, Signal) {
    (
        metadata[..ADDR_KEY_SIZE].try_into().unwrap(),
        u16::from_be_bytes(metadata[ADDR_KEY_SIZE..20].try_into().unwrap()) as usize,
        match metadata[20] {
            1 => Signal::Alive,
            _ => Signal::Dead,
        },
    )
}

pub async fn read_and_parse_metadata(
    reader: &mut RecvStream
) -> Result<([u8; ADDR_KEY_SIZE], usize, Signal), ReadExactError> {
    // Read the metadata.
    // `[..16]`: The local IP address on that `Endpoint` local machine.
    // `[16..18]`: The local port on that `Endpoint` local machine.
    // `[18..20]`: The length of the packet.
    // `[20]`: If it's 1, the client or server is telling that it's still alive.
    let mut metadata = [0_u8; METADATA_SIZE];
    if let Err(message) = reader.read_exact(&mut metadata).await {
        return Err(message);
    }

    Ok(parse_metadata(&metadata))
}
