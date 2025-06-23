# Filum

Binds locally hosted TCP/UDP server to peer-to-peer.

## What is this?

Basically, Filum allows you to "open a port" on demand using P2P, that people can connect to using this same program.

Written in Rust, running Iroh under the hood. This is a thin program on top to redirect packets between server/client over P2P if possible. Iroh will use its public relay server if a P2P connection cannot be made.

This program is best suited for testing or hosting a game server without the hassle of port forwarding and dynamic public IP stuff.

| Client support: | Server support: |
|-|-|
| ✅ TCP | ✅ TCP |
| ❌ UDP | ✅ UDP |

## Usage

On both sides, one single command is enough to get things started.

### For server:

```
filum host tcp 127.0.0.1:<port>
```

This command will then display an ID that you will share it to client, you can send it via mails, pigeons, or anything you prefer.

```
> Service started, you can now share this ID to client to let them connect to 127.0.0.1:<port>.
ID: <141 characters, odd choice>
```

### For client:

After getting the ID, put it in filum to bridge the gap between two networks:

```
filum client tcp <141 characters, odd choice> <port>
```

The `port` argument is the port that filum will use on client to make an entry point, the client will use this port to connect to the destination server, so it's merely a mimic port that spit every thing back to the server.

## Example

I've made a Colab notebook to create a filum server. Download the official filum binary, and you can try it for yourself!

https://colab.research.google.com/drive/1Wf4nEgXwUFckM1qkvUYPyESHvqwiRY2P?usp=sharing
