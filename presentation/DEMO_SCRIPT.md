# KAYA CLI Demo Script

This script is written for three terminals on the same machine or three machines on the same LAN.

## Before The Demo

Build once:

```bash
cargo build --release
```

If this is the first run for each local profile, enter the requested callsign when prompted.

Recommended callsigns:

- node 1: `Helio`
- node 2: `Ana`
- node 3: `Bruno`

## Terminal 1

Start node 1:

```bash
./scripts/run-local-lab.sh helio
```

Inside KAYA:

```text
> /join semana-info
> /status
```

## Terminal 2

Start node 2:

```bash
./scripts/run-local-lab.sh ana
```

Inside KAYA:

```text
> /join semana-info
> /who
```

## Terminal 3

Start node 3:

```bash
./scripts/run-local-lab.sh bruno
```

Inside KAYA:

```text
> /join semana-info
> /who
```

## Public Room Message

From terminal 1:

```text
> /room alguem recebe na semana-info?
```

From terminal 2:

```text
> /room recebido da Ana
```

## Trust Flow

From terminal 1:

```text
> /peers --fingerprints
> /trust Ana
> /trust Bruno
> /trust-list
```

## Secure DM

From terminal 3:

```text
> /secure-msg Helio teste privado cifrado
```

From terminal 1:

```text
> /sessions
> /dm-history Bruno
```

## File Offer

From terminal 2:

```text
> /send Helio ./docs/PROTOCOL.md
```

From terminal 1:

```text
> /files
> /accept-file <file_id>
> /file-info <file_id>
```

Use the file id shown in the incoming file-offer modal or system log.

## Mesh Route

Preferred live path:

From terminal 1:

```text
> /routes
> /route Bruno
> /mesh-status
```

Presentation fallback if live topology does not expose a visible relay path:

```text
> /demo-mesh-route
```

## Final Status

From terminal 1:

```text
> /status
> /routes
> /sessions
> /files
```

## Demo-Safe Fallback

If you need a deterministic presentation instead of live peer behavior:

```bash
./scripts/run-demo.sh banca
```

Then inside KAYA:

```text
> /demo-peers 4
> /demo-message semana-info 4
> /demo-file-offer
> /demo-security-warning
> /demo-mesh-route
> /status
```
