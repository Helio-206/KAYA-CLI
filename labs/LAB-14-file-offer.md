# LAB-14 File Offer

## Objetivo

Validar que um peer consegue oferecer um ficheiro sem envio automático.

## Setup

Abra dois terminais:

```bash
KAYA_HOME=/tmp/kaya-helio cargo run -p kaya-app --bin kaya
KAYA_HOME=/tmp/kaya-ana cargo run -p kaya-app --bin kaya
```

Use callsigns `Helio` e `Ana`.

## Comandos

Em Helio:

```text
/send Ana ./docs/PROTOCOL.md
```

Em Ana:

```text
/files
/file-info <file_id>
```

## Comportamento Esperado

- Ana recebe mensagem de sistema com nome, tamanho, fingerprint e `file_id`.
- O ficheiro fica em estado `offered`.
- Nada é salvo em `completed/` antes de `/accept-file`.

## Troubleshooting

- Se Ana não vê a oferta, confirme `/peers --fingerprints`.
- Se o path falhar, use um path local existente no terminal de Helio.
