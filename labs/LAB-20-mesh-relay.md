# LAB-20 Mesh Relay

## Objective

Verify that a node can relay a mesh envelope and that relay diagnostics are visible.

## Setup

Use three local identities:

```bash
KAYA_HOME=/tmp/kaya-helio cargo run -p kaya-app --bin kaya
KAYA_HOME=/tmp/kaya-ana cargo run -p kaya-app --bin kaya
KAYA_HOME=/tmp/kaya-bruno cargo run -p kaya-app --bin kaya
```

## Topology

Target topology:

```text
Helio -> Ana -> Bruno
```

In a flat multicast lab all nodes may still be direct. To exercise the relay path deterministically, use `/routes` and `/route <node>` to confirm route state, then test against a route that is not currently direct in a segmented lab or simulator.

## Commands

Helio:

```text
> /routes
> /route KY-A91C0D
> /msg Bruno teste via mesh
```

Ana:

```text
> /mesh-status
```

## Expected Result

- Helio sends directly if Bruno is online direct.
- If Bruno is only reachable through Ana, Helio wraps the DM in `MESH_RELAY`.
- Ana logs `MeshPacketRelayed`.
- Mesh counters show relayed packets and current route trace.

## Troubleshooting

- If the message sends direct, the LAN topology is too flat for a real relay test.
- Check that `[mesh].enabled = true` in all `config.toml` files.
- Use `/mesh-clear` and wait for route announcements to repopulate the table.
