# LAB-03: Private Messaging

Goal: verify direct messages by callsign or node id.

## Steps

Start three terminals with callsigns:

- `Helio`
- `Ana`
- `Bruno`

From Bruno:

```text
> /msg Helio teste privado
```

From Helio:

```text
> /msg Bruno recebido em privado
```

## Expected Result

Only the target terminal should display the incoming DM.

```text
[DM] Bruno -> Helio: teste privado
```

## Notes

Phase 1 DMs are private in routing only. They are not encrypted yet.
