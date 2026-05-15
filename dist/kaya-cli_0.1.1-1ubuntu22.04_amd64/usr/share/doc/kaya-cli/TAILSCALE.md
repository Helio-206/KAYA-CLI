# Tailscale Guide

Tailscale is the recommended way to test KAYA with a friend when both devices are not on the same LAN or when multicast is blocked.

KAYA does not need public internet exposure for this flow. Tailscale provides a private `100.x.x.x` address between devices, and KAYA opens a direct TCP session over it.

## Setup

Both users:

```bash
tailscale status
```

Linux host:

```bash
tailscale ip -4
```

Example output:

```text
100.81.167.95
```

## Host Side

Start KAYA:

```bash
kaya
```

Inside KAYA:

```text
> /listen 7777
> /listen-status
```

Expected:

```text
listening for direct peers on 0.0.0.0:7777
```

If the listener starts but the friend cannot connect, check the OS firewall for TCP port `7777`.

## Friend Side

Start KAYA:

```bash
kaya.exe
```

Inside KAYA:

```text
> /connect 100.81.167.95:7777
> /connections
```

Expected:

```text
[DIRECT] Connected to Helio KY-XXXXXX
```

## Validation Flow

Host:

```text
> /who
> /trust Ana
> /secure-msg Ana teste seguro via tailscale
> /send Ana ./docs/PROTOCOL.md
```

Friend:

```text
> /who
> /trust Helio
> /secure-msg Helio recebido seguro
> /accept-file <file_id>
> /files
```

## Troubleshooting

`direct connect failed: connection refused`

- KAYA is not listening on the host.
- Run `/listen 7777` again.
- Confirm the IP and port.

`direct connect failed: timed out`

- Tailscale is not connected or ACLs block the devices.
- Check `tailscale status` on both machines.
- Check the host firewall.

Peer connects but secure DM fails:

- Wait for the signed `HELLO` to register.
- Run `/peers --fingerprints`.
- Trust the peer with `/trust <callsign>`.

File offer works but chunks fail:

- Use direct TCP, not mesh.
- Run `/connections` and confirm `direct_tcp`.
