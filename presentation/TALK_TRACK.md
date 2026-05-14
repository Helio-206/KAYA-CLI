# KAYA CLI Talk Track

Estimated duration: 5 to 7 minutes.

## Opening

KAYA CLI is an offline-first communication system for local networks. The core idea is simple: if people are physically together on the same LAN, that should be enough to create a temporary digital community. They should be able to discover each other, join rooms, exchange messages, send files, and even use encrypted direct messages without depending on the internet, a cloud service, or a central server.

## Problem

Most communication products assume stable internet and permanent infrastructure. That works until it does not. In workshops, campus events, local operations, temporary response teams, and other short-lived physical gatherings, the group often needs digital coordination exactly when centralized connectivity is weak, unavailable, or not desirable.

KAYA starts from a different assumption: proximity matters. A local network is not just plumbing. It can itself become the communication space.

## What KAYA Does

In the current release candidate, KAYA discovers peers over the local network, supports rooms, direct messages, encrypted direct messages, file offers and transfer, trust fingerprints, and an experimental mesh relay layer. The interface is terminal-first, which is intentional. It keeps the system lightweight, inspectable, and easy to reason about during technical evaluation.

## Demo

In the demo, I start three nodes on the same local network. They join the same room, exchange public room messages, and then I move into identity and trust. I can inspect peer fingerprints, mark peers as trusted, and establish a secure direct-message session. Then I show a file offer flow and a mesh route view. If the live network topology is too clean to make relay behavior obvious, the built-in demo mode gives a deterministic mesh-route demonstration without changing the protocol.

## How It Works

Architecturally, KAYA is a Rust workspace with clear crate boundaries. The app crate coordinates runtime behavior. The transport crate handles UDP multicast. The protocol crate defines packets. The peer and rooms crates maintain social state. The security crate handles identity, signatures, trust, and encrypted DM sessions. The files crate manages chunking and integrity. The mesh crate handles route announcements, route requests, relay rules, TTL, and diagnostics. The UI crate renders everything in a ratatui terminal interface.

There is also an internal event bus so packet handling, domain logic, and UI projection remain decoupled. That makes the system easier to explain and easier to evolve.

## Why It Is Interesting

The interesting claim here is not only that KAYA works. It is that local-first communication can be treated as a real product direction rather than a degraded fallback. KAYA combines systems engineering, human coordination, and a clear network model around that idea.

## Honest Limits

It is important not to oversell the project. Today KAYA is limited to LAN environments that allow IPv4 UDP multicast. The mesh layer is experimental. Public rooms are plaintext. File chunks are not yet relayed over mesh. There is no NAT traversal, no mobile client, and no external cryptographic audit yet.

So this is not presented as a production-ready messenger. It is presented as a strong release candidate, a convincing technical prototype, and a disciplined foundation for future work.

## Roadmap

The public roadmap is intentionally staged. The next major areas are a binary protocol, a future transport evolution toward QUIC, mobile and Android support, and then a production security review. Each step is meant to harden a proven boundary instead of adding complexity just for presentation value.

## Closing

The core message is this: when people share a place, they should be able to create useful digital coordination locally, without asking a distant server for permission. KAYA CLI is a concrete, working argument for that idea.
