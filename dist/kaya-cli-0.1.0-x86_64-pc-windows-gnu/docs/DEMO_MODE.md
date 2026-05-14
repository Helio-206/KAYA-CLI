# KAYA Demo Mode

Demo mode is the fastest way to present KAYA without polluting a real operator profile.

## Goals

- isolate config, identity, history, trust state, and downloads from the default profile
- avoid an interactive nickname prompt by generating a demo-friendly callsign when needed
- provide deterministic commands that populate peers, traffic, mesh traces, file offers, and trust warnings

## Start

```bash
cargo run -p kaya-app --bin kaya -- --demo --profile demo
```

Or use the helper script:

```bash
./scripts/run-demo.sh presenter
```

The helper starts KAYA with an isolated data directory under `/tmp` and keeps the demo profile out of `~/.kaya`.

## Demo Commands

- `/demo-reset` resets in-memory demo content and returns to the default room view
- `/demo-peers <n>` seeds visible peers
- `/demo-message <room> <count>` seeds room traffic
- `/demo-mesh-route` shows a secure multi-hop delivery example
- `/demo-file-offer` opens a file-offer modal and inserts a matching system event
- `/demo-security-warning` opens a trust-warning modal and increments security warnings

## Suggested Walkthrough

```text
> /demo-peers 4
> /demo-message semana-info 4
> /demo-mesh-route
> /demo-file-offer
> /demo-security-warning
```

## Presentation Notes

- open with the splash screen and show that the interface starts without an internet dependency
- seed peers first so the network and identity panels look alive before messaging
- use `/demo-mesh-route` to highlight secure relay and route tracing in one step
- use `/demo-file-offer` and `/demo-security-warning` to demonstrate the new modal overlays
- finish with `/status`, `/routes`, or `/sessions` depending on the audience focus

## Profiles

- `default`: regular operator profile under `~/.kaya`
- `demo`: isolated presentation profile with demo defaults
- `lab`: longer tolerances for local experiments
- `paranoid`: stricter file-transfer and relay policy defaults
