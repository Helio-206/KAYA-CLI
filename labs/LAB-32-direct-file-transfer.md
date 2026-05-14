# LAB-32 Direct File Transfer

## Objetivo

Validar oferta e envio de ficheiro sobre `direct_tcp`, incluindo chunks e hash final.

## Setup

- Conexão direta ativa.
- Um ficheiro pequeno disponível para teste.
- File transfer habilitado no config.

## Comandos

Sender:

```text
> /connections
> /send Ana ./docs/PROTOCOL.md
> /files
```

Receiver:

```text
> /files
> /accept-file <file_id>
> /files
> /open-folder
```

## Comportamento Esperado

- Receiver vê a oferta com callsign, tamanho e segurança.
- Sender envia chunks por `direct_tcp`.
- Receiver valida hash final.
- Ficheiro final aparece em `~/.kaya/files/completed`.

## Troubleshooting

- Se aparecer `file chunks over mesh not enabled yet`, não há conexão direta ativa.
- Execute `/connections` em ambos os lados.
- Se hash falhar, repita o envio e verifique se algum firewall/antivírus encerrou a sessão.
