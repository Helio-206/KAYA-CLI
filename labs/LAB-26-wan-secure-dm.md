# LAB-26 WAN Secure DM

## Goal

Verify encrypted DM over relay between nodes in different networks.

## Steps

1. Start a relay reachable from both operators.
2. Start two nodes with `--relay tcp://HOST:7777`.
3. On node A, run `/relay-peers`.
4. On node A, run `/secure-msg <peer> segredo por relay`.
5. On node B, confirm delivery and trust state.

## Expected Result

- a secure session is established
- the DM is delivered over relay
- fingerprints and trust checks still behave normally