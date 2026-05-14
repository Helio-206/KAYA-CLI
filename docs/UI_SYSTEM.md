# KAYA UI System

The TUI lives in `crates/ui` and uses ratatui with crossterm.

## Panels

- Header: space, current room, node id, callsign, status.
- Left panel: known rooms and membership counts.
- Center panel: room chat and system messages.
- Right panel: current-room members, global peers, and DMs.
- Network: peers, latency, packet counters, byte counters, uptime, heartbeat, timeout, render timing, memory, event counters, security summary, and logs.
- Input: command/message entry.

## Security Indicators

- Header/network projection includes the local identity fingerprint.
- Peer rows include short fingerprints and trust state when available.
- Encrypted DMs render with a `[SECURE]` marker.
- The technical panel shows trusted peers, blocked peers, active secure sessions, and security warning count.

## Controls

- `Enter`: submit input.
- `Up` / `Down`: command history.
- `PageUp` / `PageDown`: traffic scroll.
- `Ctrl-L`: clear traffic.
- `Ctrl-C` or `Esc`: exit.

## Design Direction

The UI should read as a Linux infrastructure console:

- restrained dark palette;
- technical cyan/blue accents;
- compact operational data;
- no exaggerated neon or game-like styling.

## Separation

The UI does not own networking, protocol parsing, peer discovery, or room routing. It renders `UiState`, which is updated by the runtime from internal events.
