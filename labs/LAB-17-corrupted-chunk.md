# LAB-17 Corrupted Chunk

## Objetivo

Validar comportamento esperado quando um chunk ou payload final é corrompido.

## Setup

Este lab é mais simples com teste automatizado:

```bash
cargo test -p kaya-files corrupted_chunk_is_rejected
```

## Comandos

Para observação manual, rode uma transferência normal e acompanhe logs:

```text
/send Ana ./docs/PROTOCOL.md
/accept-file <file_id>
/files
```

## Comportamento Esperado

- Chunk com hash inválido é rejeitado.
- Transferência corrompida não é salva como ficheiro final válido.
- Eventos `FileHashMismatch` e `FileTransferFailed` são emitidos.

## Troubleshooting

- UDP real pode perder pacotes; isso não é corrupção criptográfica.
- Retransmissão e resume ficam para fase futura.
