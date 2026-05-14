# LAB-06: Room Membership

Goal: verify room creation, announcements, joins, leaves, and member counts.

## Setup

Open three terminals:

```bash
KAYA_HOME=/tmp/kaya-helio cargo run -p kaya-app --bin kaya
KAYA_HOME=/tmp/kaya-ana cargo run -p kaya-app --bin kaya
KAYA_HOME=/tmp/kaya-bruno cargo run -p kaya-app --bin kaya
```

Use callsigns:

- `Helio`
- `Ana`
- `Bruno`

## Steps

Helio:

```text
> /create semana-info
> /join semana-info
> /room sistema online
```

Ana:

```text
> /rooms
> /join semana-info
> /room recebido
```

Bruno:

```text
> /rooms
> /who
```

## Expected Behavior

- `#semana-info` appears in room lists after announcement.
- Members panel shows joined nodes for the current room.
- Member counts increase as peers join.
- `/leave semana-info` removes the local node from the room.

## Troubleshooting

- Confirm all nodes use the same multicast address and port.
- Wait one heartbeat interval and run `/rooms` again.
- Check technical logs for `ROOM_ANNOUNCE`, `ROOM_JOIN`, or member sync errors.
