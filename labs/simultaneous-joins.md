# LAB: Simultaneous Joins

Goal: verify multiple peers joining the same room converge locally.

## Steps

Start three nodes, then quickly run:

```text
> /join semana-info
```

on each terminal.

## Expected Behavior

- The room appears once in `/rooms`.
- Peers appear once in `/who`.
- Room messages are visible to peers in the same room.
- Heartbeats do not spam room-join logs.
