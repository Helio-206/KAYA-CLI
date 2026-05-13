# LAB: Malformed Packets

Goal: verify malformed datagrams are rejected at the protocol boundary.

## Simulation

From another terminal, send invalid JSON to the multicast group with any UDP tool available on the machine.

Example with `nc` variants that support UDP:

```bash
printf '{bad-json' | nc -u 239.71.0.1 42424
```

## Expected Behavior

- KAYA does not crash.
- The technical panel records an error event.
- The malformed counter increases.
- No peer or room state is mutated.
