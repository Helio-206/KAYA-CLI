# LAB-13 Blocked Peers

## Objetivo

Validar que peers bloqueados deixam de aparecer como participantes úteis e têm pacotes rejeitados antes do chat.

## Setup

Abra três terminais:

```bash
KAYA_HOME=/tmp/kaya-helio cargo run -p kaya-app --bin kaya
KAYA_HOME=/tmp/kaya-ana cargo run -p kaya-app --bin kaya
KAYA_HOME=/tmp/kaya-bruno cargo run -p kaya-app --bin kaya
```

Use callsigns `Helio`, `Ana` e `Bruno`.

## Passos

1. Aguarde discovery.
2. Em Helio, execute `/peers --fingerprints`.
3. Em Helio, execute `/block Bruno`.
4. Em Bruno, envie uma mensagem de sala.
5. Em Bruno, envie `/msg Helio teste`.
6. Em Helio, execute `/trust-list`.

## Resultado Esperado

- Bruno aparece como `blocked` no trust store de Helio.
- Pacotes de Bruno geram security warning em Helio.
- Mensagens de Bruno não entram no chat de Helio depois do bloqueio.
- Ana continua visível e funcional.

## Troubleshooting

- Bloqueio é local: se Ana não bloqueou Bruno, Ana ainda poderá ver Bruno.
- Se Bruno ainda aparece, confirme que `/block` foi executado no terminal correto e que o target não era um callsign ambíguo.
