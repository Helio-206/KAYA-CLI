# LAB-16 Encrypted File Transfer

## Objetivo

Validar transferência de ficheiro usando sessão segura existente.

## Setup

```bash
KAYA_HOME=/tmp/kaya-helio cargo run -p kaya-app --bin kaya
KAYA_HOME=/tmp/kaya-ana cargo run -p kaya-app --bin kaya
```

## Comandos

Em Helio:

```text
/trust Ana
/secure-msg Ana teste seguro
/sessions
/send Ana ./docs/PROTOCOL.md
```

Em Ana:

```text
/accept-file <file_id>
/files
```

## Comportamento Esperado

- `/sessions` mostra sessão ativa.
- A oferta aparece como `encrypted`.
- Chunks usam `FILE_CHUNK_ENCRYPTED`.
- O ficheiro final passa na validação SHA-256.

## Troubleshooting

- Se a oferta aparecer `unencrypted`, a sessão segura ainda não estava ativa.
- Envie `/secure-msg Ana ping` e aguarde `secure session active`.
