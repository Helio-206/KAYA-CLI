# KAYA Daemon Design

This document prepares the daemon layer for a later phase. It does not implement the daemon yet.

## Goal

Run a single KAYA node in the background and let multiple local applications talk to it through a local socket.

## Intended model

- `kaya-daemon` owns transport, identity, mesh, files, and persistence
- CLI becomes a thin daemon client
- desktop apps, bots, and services connect to the same local daemon

## Local control channel

Planned options:

- Unix domain socket on Linux/macOS
- named pipe on Windows

## API shape

The daemon API should mirror the SDK shape:

- start and stop node
- set callsign
- join room
- send room message
- send DM and secure DM
- send file
- inspect peers, sessions, routes, and transfers
- subscribe to event stream

## Why not now

Phase 8 only establishes:

- release packaging
- public `kaya-core`
- official `kaya-sdk`
- embedding docs and examples

The daemon becomes easier after the headless core is stable.