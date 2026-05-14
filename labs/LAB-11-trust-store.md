# LAB-11 Trust Store

## Objetivo

Validar estados `unknown`, `trusted` e `blocked` no trust store local.

## Setup

Abra dois terminais:

```bash
KAYA_HOME=/tmp/kaya-helio cargo run -p kaya-app --bin kaya
KAYA_HOME=/tmp/kaya-ana cargo run -p kaya-app --bin kaya
```

## Passos

1. Aguarde discovery.
2. Em Helio, execute `/peers --fingerprints`.
3. Execute `/trust Ana`.
4. Execute `/trust-list`.
5. Execute `/untrust Ana`.
6. Execute `/trust-list` novamente.

## Resultado Esperado

- Ana entra primeiro como `unknown`.
- Depois de `/trust Ana`, Ana aparece como `trusted`.
- Depois de `/untrust Ana`, Ana volta para `unknown`.
- O ficheiro `$KAYA_HOME/trust.toml` registra o peer, fingerprint, `first_seen`, `last_seen` e estado.

## Troubleshooting

- Se `/trust Ana` disser que não há fingerprint, aguarde um heartbeat assinado ou execute `/fingerprint` no terminal remoto para confirmar identidade.
- Se houver callsigns duplicados, use o `node_id` mostrado em `/peers --fingerprints`.
