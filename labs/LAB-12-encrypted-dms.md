# LAB-12 Encrypted DMs

## Objetivo

Validar handshake de sessão segura e envio de `DIRECT_MESSAGE_ENCRYPTED`.

## Setup

Abra dois terminais:

```bash
KAYA_HOME=/tmp/kaya-helio cargo run -p kaya-app --bin kaya
KAYA_HOME=/tmp/kaya-ana cargo run -p kaya-app --bin kaya
```

Use callsigns `Helio` e `Ana`.

## Passos

1. Em Helio, execute `/peers --fingerprints`.
2. Opcionalmente execute `/trust Ana`.
3. Em Helio, execute `/secure-msg Ana segredo local`.
4. Em ambos os terminais, execute `/sessions`.
5. Envie outra mensagem: `/secure-msg Ana segunda mensagem`.

## Resultado Esperado

- A primeira mensagem cria `DM_SESSION_REQUEST` e fica em fila até `DM_SESSION_ACCEPT`.
- A sessão muda para `active`.
- A mensagem aparece como `[SECURE] Helio -> Ana`.
- `/sessions` mostra session id, peer e contador de mensagens.

## Troubleshooting

- Se a primeira mensagem não chegar, confirme que Ana não bloqueou Helio.
- Se a sessão não ativar, olhe o painel de logs para `security warning`.
- Para reiniciar a sessão, use `/close-session Ana` e depois `/secure-msg Ana novo teste`.
