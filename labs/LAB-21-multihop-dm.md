# LAB-21 Multihop DM

## Objective

Verify that plaintext and encrypted direct messages can use a mesh route when the destination is not directly online.

## Setup

Start Helio, Ana, and Bruno with separate identities. Trust fingerprints first if testing encrypted DMs.

```text
Helio: /trust Ana
Ana:   /trust Bruno
Bruno: /trust Ana
```

## Topology

```text
Helio knows Ana.
Ana knows Bruno.
Helio does not know Bruno directly.
```

This topology requires a segmented lab, network namespaces, or a simulator because a single flat multicast LAN tends to make every peer direct.

## Commands

Helio:

```text
> /routes
> /secure-msg Bruno teste via mesh
```

Ana:

```text
> /mesh-status
```

Bruno:

```text
> /sessions
> /dm-history Helio
```

## Expected Result

- Helio uses an existing route or emits `ROUTE_REQUEST`.
- Ana relays without decrypting the encrypted DM payload.
- Bruno receives `[SECURE] Helio -> Bruno`.
- The mesh panel shows a route trace.

## Troubleshooting

- If Helio reports no route, wait for `ROUTE_RESPONSE` or retry after `/routes`.
- If secure session setup stalls, verify each peer has seen signed packets from the other side.
- For plaintext `/msg`, remember relay nodes can read the inner `DIRECT_MESSAGE`; use `/secure-msg` for sensitive content.
