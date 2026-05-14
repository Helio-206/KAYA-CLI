# LAB-09: History

Goal: verify local room and DM history.

## Steps

Helio:

```text
> /join semana-info
> /room primeira mensagem
> /history semana-info
```

Bruno:

```text
> /msg Helio teste privado
```

Helio:

```text
> /dm-history Bruno
```

## Expected Behavior

- `/history [room]` shows local room messages stored on this node.
- `/dm-history <peer>` shows local DMs with that peer.
- History is local-only; it is not a replicated archive.

## Troubleshooting

- Confirm `KAYA_HOME` points to a writable directory.
- History is filtered by normalized room names.
- DMs are filtered by callsign or target field stored locally.
