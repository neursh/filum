# Proxy process

On both TCP and UDP, instance and host will connect to each other over Iroh's `Endpoint` with bidirectional communication.

For each client on instance side, is each `Endpoint` connected to the host.

# TCP

Upon client connection, instance will request a new `Endpoint` to the host, ensuring each socket has its own space for transfering packets.

Next, the instance will try to connect to the host's `Endpoint`, if it's available, send a ping `[1]` message, and wait for response.

The host receiving the request and ping message, send back a pong `[1]` message back to instance, completing the base layer to transport packets.

When the instance receives the pong message, both sides will start plugging in the `Endpoint` to relay the actual connection between client and server, completing the connection between client and server.

# UDP

Since the nature of UDP is connectionless, I've come up the idea of using `moka`'s `Cache` to store the `Endpoint`, expiring after 5 minutes if no new packets sent.

Both instance and host will hold an `Endpoint` for each new IP address found while listening for UDP packets.

On instance side. If a new IP is detected, create a new tokio task and a new mpsc channel for it. Inside the task, initiate a new `Endpoint` to host. Once connected, listen for message from mpsc channel.

In the mean time, put everything inside that mpsc channel when the same IP send another bunch of packets, we'll handle it later, so no race condition!

On host side. If a new `Endpoint` is requesting, accept it, and then create a new UdpSocket, spawning two tasks:

- Stream back to server from `Endpoint`.
- Stream from server to `Endpoint`.
