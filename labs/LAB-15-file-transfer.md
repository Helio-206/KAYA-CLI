# LAB-15 File Transfer

## Objetivo

Validar oferta, aceitação, chunking, ACKs, reassembly e hash final.

## Setup

```bash
KAYA_HOME=/tmp/kaya-helio cargo run -p kaya-app --bin kaya
KAYA_HOME=/tmp/kaya-ana cargo run -p kaya-app --bin kaya
```

## Comandos

Em Helio:

```text
/send Ana ./docs/PROTOCOL.md
```

Em Ana:

```text
/accept-file <file_id>
/files
/open-folder
```

## Comportamento Esperado

- Helio envia `FILE_OFFER`.
- Ana aceita com `FILE_ACCEPT`.
- Helio envia `FILE_CHUNK`.
- Ana valida chunks, reconstrói o ficheiro e grava em `~/.kaya/files/completed`.
- `/files` mostra `completed`.

## Troubleshooting

- Se a transferência ficar parada, confira os logs técnicos.
- Se o hash falhar, o estado deve ser `corrupted` ou `failed`.
