# LAB-25 ngrok Relay

## Goal

Verify that a public tunnel can expose the KAYA relay to a remote operator.

## Steps

1. Run `kaya relay --bind 127.0.0.1:7777`.
2. Run `ngrok tcp 7777`.
3. Copy the tunnel endpoint.
4. Start one remote node with `--relay tcp://TUNNEL-HOST:TUNNEL-PORT`.
5. Run `/relay-status` and `/relay-peers`.

## Expected Result

- the remote node connects
- relay peer list becomes non-empty
- DM through `/msg` reaches the remote node