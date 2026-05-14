# LAB-08: Presence

Goal: verify peer presence states.

## States

- `online`
- `away`
- `busy`
- `invisible`
- derived `offline`

## Steps

Bruno:

```text
> /presence busy
```

Ana:

```text
> /who
```

Helio:

```text
> /presence away
```

## Expected Behavior

- The peers panel shows updated presence beside callsigns.
- `/who` includes presence and online/offline state.
- Closing a node or waiting for timeout derives `offline`.

## Troubleshooting

- Presence is delivered over multicast; wait one heartbeat interval.
- `offline` cannot be set manually with `/presence offline`.
- Check logs for `PRESENCE_UPDATE`.
