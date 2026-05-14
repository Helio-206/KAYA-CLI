# KAYA Friend Quickstart

This package is enough to test KAYA without cloning the repository.

## What You Received

- `bin/kaya`: the compiled CLI binary
- `docs/`: basic usage and WAN relay notes
- `scripts/`: helper scripts bundled with the release

## Install

Option 1, run from the extracted folder:

```bash
./bin/kaya --version
```

Option 2, install globally:

```bash
sudo install -m 0755 ./bin/kaya /usr/local/bin/kaya
kaya --version
```

## Join a Remote Test

If the sender gave you a relay endpoint such as `tcp://HOST:PORT`, start KAYA with:

```bash
./bin/kaya --relay tcp://HOST:PORT
```

If you installed globally:

```bash
kaya --relay tcp://HOST:PORT
```

## First Commands Inside KAYA

```text
/relay-status
/relay-peers
/join semana-info
olá, estou aqui
/msg CALLSIGN mensagem privada
/secure-msg CALLSIGN mensagem cifrada
```

## Notes

- Use the same KAYA version as the other operator.
- `secure-msg` is end-to-end encrypted.
- Room messages over relay are not end-to-end encrypted.
- If `CALLSIGN` is ambiguous, use the peer node id shown by `/relay-peers`.