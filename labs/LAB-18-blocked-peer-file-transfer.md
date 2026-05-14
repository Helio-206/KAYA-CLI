# LAB-18 Blocked Peer File Transfer

## Objetivo

Garantir que peers bloqueados não conseguem enviar ofertas ou chunks úteis.

## Setup

```bash
KAYA_HOME=/tmp/kaya-helio cargo run -p kaya-app --bin kaya
KAYA_HOME=/tmp/kaya-bruno cargo run -p kaya-app --bin kaya
```

## Comandos

Em Helio:

```text
/peers --fingerprints
/block Bruno
/trust-list
```

Em Bruno:

```text
/send Helio ./docs/PROTOCOL.md
```

## Comportamento Esperado

- Helio marca Bruno como `blocked`.
- Pacotes de Bruno geram security warning.
- A oferta de ficheiro não entra como transferência aceite ou visível para ação.

## Troubleshooting

- Bloqueio é local. Outros peers continuam vendo Bruno se não o bloquearem.
- Se houver callsign duplicado, bloqueie pelo `node_id`.
