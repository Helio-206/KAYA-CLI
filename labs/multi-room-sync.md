# LAB: Multi-Room Sync

Goal: observe behavior across several rooms.

## Steps

Terminal 1:

```text
> /join geral
> geral online
```

Terminal 2:

```text
> /join ops
> ops online
```

Terminal 3:

```text
> /join geral
> recebido no geral
```

## Expected Behavior

- Incoming messages for non-current rooms update state but do not force local room switches.
- `/rooms` lists rooms observed locally.
- The traffic panel focuses on the current room plus DMs/system messages.
