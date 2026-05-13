# LAB-04: Node Failure

Goal: verify peer timeout behavior.

## Steps

1. Start two KAYA terminals.
2. Confirm `/who` shows the peer online.
3. Close one terminal or kill the process.
4. Wait longer than the peer timeout window.

Default timeout:

```text
12 seconds
```

## Expected Result

The remaining terminal logs a peer timeout and `/who` shows the peer offline.

```text
log: peer timeout KY-XXXXXX
```

## Notes

Graceful `/exit` sends `LEAVE`. Process failure is detected through missing heartbeats.
