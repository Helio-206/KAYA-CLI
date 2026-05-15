# KAYA Relay Through ngrok

`ngrok` is not used to tunnel KAYA multicast. It is only useful to expose the TCP relay service.

## Correct Pattern

1. Start the KAYA relay locally:

```bash
cargo run -p kaya-app --bin kaya -- relay --bind 127.0.0.1:7777
```

2. Expose the relay port:

```bash
ngrok tcp 7777
```

3. Give the generated `tcp://HOST:PORT` endpoint to the remote operator.

4. Remote nodes connect with:

```bash
cargo run -p kaya-app --bin kaya -- --relay tcp://HOST:PORT
```

## What ngrok Does Not Solve

- It does not make UDP multicast work across homes.
- It does not replace the relay server.
- It does not add end-to-end encryption by itself.

Use it only as a public entry point for the relay process.