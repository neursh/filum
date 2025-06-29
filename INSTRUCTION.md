# Proxy process

On both TCP and UDP, instance and host will connect to each other over Iroh's `Endpoint` with bidirectional communication.

One instance will send over all connections to host.

To send a packet over, compose the header, consuming 20 bytes

[0..16]: The local IP address on that `Endpoint` local machine.

[16..18]: The local port on that `Endpoint` local machine.

[18..20]: The length of the packet.
