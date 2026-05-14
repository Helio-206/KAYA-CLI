# LAB-31 Direct Secure DM

## Objetivo

Validar DMs encriptadas sobre conexão `direct_tcp`.

## Setup

- LAB-29 ou LAB-30 concluído.
- Ambos os peers aparecem em `/connections`.

## Comandos

Peer A:

```text
> /peers --fingerprints
> /trust Ana
> /secure-msg Ana teste seguro via direct
> /sessions
```

Peer B:

```text
> /peers --fingerprints
> /trust Helio
> /secure-msg Helio recebido seguro
> /sessions
```

## Comportamento Esperado

- A sessão segura é criada com `DM_SESSION_REQUEST` e `DM_SESSION_ACCEPT`.
- A mensagem aparece como `[SECURE]`.
- `/sessions` mostra sessão ativa.
- O painel `CONNECTIONS` continua indicando `direct_tcp`.

## Troubleshooting

- Se a sessão não inicia, aguarde o `HELLO` assinado e execute `/peers --fingerprints`.
- Se houver fingerprint inesperado, não confie no peer até validar fora de banda.
- Se `/secure-msg` cair para mesh/relay, confirme `/connections`.
