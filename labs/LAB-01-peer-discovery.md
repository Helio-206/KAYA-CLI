# LAB-01: Peer Discovery

Goal: verify that two KAYA nodes discover each other on the same LAN.

## Setup

Open two terminals on the same machine or two machines on the same local network.

Terminal 1:

```bash
KAYA_HOME=/tmp/kaya-helio cargo run -p kaya-app --bin kaya
```

Callsign:

```text
Helio
```

Terminal 2:

```bash
KAYA_HOME=/tmp/kaya-ana cargo run -p kaya-app --bin kaya
```

Callsign:

```text
Ana
```

## Expected Result

Both terminals should show peer discovery logs and `/who` should list the other node as online.

```text
> /who
Ana KY-XXXXXX online
```

## Troubleshooting

- Confirm both nodes are on the same LAN.
- Confirm the network allows IPv4 multicast.
- Try the same machine first to validate local socket reuse.
