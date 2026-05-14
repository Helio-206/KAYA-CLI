# LAB-07: Direct Messages

Goal: verify robust DM routing by callsign or node id.

## Setup

Start three terminals with callsigns `Helio`, `Ana`, and `Bruno`.

## Steps

Bruno:

```text
> /msg Helio teste privado
```

Helio:

```text
> /dm-history Bruno
```

Ana:

```text
> /msg Helio outro teste
```

## Expected Behavior

- DMs appear in the right-side `DMS` panel.
- Public room chat stays separate from DMs.
- `/dm-history <peer>` shows local DM history.
- If two peers share the same callsign, KAYA rejects `/msg <callsign>` and asks for a node id.
- If a peer does not exist, KAYA shows a clear target-not-found error.

## Troubleshooting

- Run `/who` to confirm callsign and node id.
- Use node id directly when callsigns are duplicated.
- Check that the target peer is online.
