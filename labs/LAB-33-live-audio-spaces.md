# LAB-33 Live Audio Spaces

## Objetivo

Validar o fluxo de entrada, presença e operação de um `voice space` por sala, incluindo mute, push-to-talk e heartbeat de voz.

## Setup

- Dois operadores ativos na mesma rede ou via relay permitido.
- Ambos já executaram `/join semana-info`.
- Voice habilitado no config padrão.
- Em Linux, `arecord` e `aplay` disponíveis no sistema.
- Em Windows, um dispositivo de entrada e saída compatível com áudio mono 8 kHz via backend nativo.
- Opcional: build com `--features kaya-voice/native-audio` se quiser testar descoberta real de dispositivos.

## Comandos

Operator A:

```text
> /join semana-info
> /voice-join semana-info
> /voice-status
> /ptt
> /voice-mute
> /voice-unmute
> /voice-leave
```

Operator B:

```text
> /join semana-info
> /voice-join semana-info
> /voice-status
```

## Comportamento Esperado

- Ambos entram no `voice space` da sala atual.
- O cabeçalho e o painel de rede mostram o estado `VOICE` com sala, mute e PTT.
- `Space` com input vazio atua como hold/release de push-to-talk.
- `/voice-status` mostra sessão, contadores de frames e speakers ativos.
- Ao sair com `/voice-leave` ou `/leave semana-info`, o estado de voz local é encerrado.

## Troubleshooting

- Se aparecer `join #<room> before /voice-join`, entre primeiro na sala de chat.
- Se aparecer `voice disabled in config`, habilite a seção `[voice]` no config.
- Se `devices=unavailable`, isso afeta apenas descoberta de dispositivos; captura/playback por `arecord/aplay` ainda pode funcionar.
- Se aparecer erro de `voice.capture` ou `voice.playback` no Linux, verifique se `arecord` e `aplay` estão instalados e acessam o dispositivo padrão.
- Se aparecer erro de `voice.capture` ou `voice.playback` no Windows, confirme se o dispositivo suporta mono 8 kHz e está acessível como dispositivo padrão ou pelo nome configurado.
- Se não houver speakers ativos, confirme que heartbeats de voz estão chegando com `/voice-status` e `/logs`.