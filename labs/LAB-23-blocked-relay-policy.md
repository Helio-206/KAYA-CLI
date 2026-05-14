# LAB-23 Blocked Relay Policy

## Objective

Verify that blocked peers cannot use the local node as a relay.

## Setup

Start three nodes with separate identities.

```bash
KAYA_HOME=/tmp/kaya-helio cargo run -p kaya-app --bin kaya
KAYA_HOME=/tmp/kaya-ana cargo run -p kaya-app --bin kaya
KAYA_HOME=/tmp/kaya-bruno cargo run -p kaya-app --bin kaya
```

## Topology

```text
Helio -> Ana -> Bruno
```

Ana is the intended relay.

## Commands

Ana:

```text
> /block Helio
> /trust-list
> /mesh-status
```

Helio:

```text
> /secure-msg Bruno teste bloqueado
```

## Expected Result

- Ana rejects packets from Helio after blocking.
- Ana logs `RelayDenied` or a security warning.
- Bruno does not receive the relayed message through Ana.
- `/mesh-status` shows increased dropped packet count if a relay attempt occurred.

## Troubleshooting

- Confirm Ana has observed Helio's fingerprint before running `/block Helio`.
- If the message still arrives, Helio may have a direct route to Bruno.
- Confirm `[mesh].allow_relay_for_blocked = false`.
