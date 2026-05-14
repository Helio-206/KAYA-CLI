# LAB-10 Identity and Fingerprints

## Objetivo

Validar que cada nó cria uma identidade persistente, anuncia pacotes assinados e expõe fingerprint público.

## Setup

Abra dois terminais com diretórios KAYA isolados:

```bash
KAYA_HOME=/tmp/kaya-helio cargo run -p kaya-app --bin kaya
KAYA_HOME=/tmp/kaya-ana cargo run -p kaya-app --bin kaya
```

Use callsigns `Helio` e `Ana`.

## Passos

1. Em cada terminal, execute `/identity`.
2. Execute `/fingerprint`.
3. Execute `/peers --fingerprints`.
4. Reinicie um terminal com o mesmo `KAYA_HOME`.
5. Execute `/fingerprint` novamente.

## Resultado Esperado

- `~/.kaya/identity.toml` ou `$KAYA_HOME/identity.toml` é criado.
- O fingerprint local tem formato `KAYA-FP: XXXX-XXXX-XXXX`.
- O fingerprint permanece igual após reiniciar com o mesmo `KAYA_HOME`.
- O peer remoto aparece com fingerprint após receber pacotes assinados.

## Troubleshooting

- Se não houver peers, confirme que ambos os terminais usam a mesma LAN e multicast.
- Se o fingerprint mudar após reiniciar, verifique se o `KAYA_HOME` usado é o mesmo.
- Nunca cole o conteúdo completo de `identity.toml` em logs ou issue trackers.
