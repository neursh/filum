# Filum

Binds locally hosted Minecraft server to peer-to-peer

This project is mainly developing for Windows platform. Once the first beta is released, I'll start working on a Linux port.

## Network instruction

Step by step instruction for creating this.

### 1. Bridge -> Postman:

- Bridge connects to postman with 2 pieces of information:
  - First u8 is role, must be 1.
  - An ASCII string of 24 character as id.
  - Private IP/ private port: `[u8, u8, u8, u8, u16]` (buffer: 6)
  - NOTE: Buffer size is 31.
  * If id already exists, just close the bridge.

### 2. Client -> Postman:

- Client connects to postman with 2 pieces of information:
  - First bit is role, must be 0.
  - An ASCII string of 24 character as id. (8 \* 24 => 192)
  - Private IP/ private port: `[u8, u8, u8, u8, u16]` (buffer: 48)
  - NOTE: Buffer size is 241.
  * If id is not found, just close the client.

### 3. Postman -> Bridge:

- Once the client has connected, postman will send the client's IP and port:
  - `[u8, u8, u8, u8, u16, u8, u8, u8, u8, u16]` (buffer: 96)
  - First `[u8, u8, u8, u8, u16]` is the public IP / public port (NAT provided).
  - Next `[u8, u8, u8, u8, u16]` is the private IP / private port.

### 4. Postman -> Client:

- When the postman receives that information, it will send back to the client, similiar packet:
  - `[u8, u8, u8, u8, u16, u8, u8, u8, u8, u16]` (buffer: 96)
  - First `[u8, u8, u8, u8, u16]` is the public IP / public port (NAT provided).
  - Next `[u8, u8, u8, u8, u16]` is the private IP / private port.

### 5. Client/Bridge -X-> Postman:

- Postman's job finished here, client and bridge disconnect from the postman, or postman just close them.

## Papers & references

[Peer-to-Peer Communication Across Network Address Translators (Apr 10 2005, by Bryan Ford)](https://bford.info/pub/net/p2pnat/)
