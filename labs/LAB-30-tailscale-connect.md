# LAB-30 Tailscale Connect

## Objetivo

Testar KAYA entre dois dispositivos usando Tailscale, sem multicast e sem relay público.

## Setup

- Ambos os dispositivos ligados à mesma tailnet.
- Tailscale autenticado nos dois lados.
- KAYA instalado nos dois lados.

Host Linux:

```bash
tailscale ip -4
```

Exemplo:

```text
100.81.167.95
```

## Comandos

Host:

```text
> /listen 7777
> /listen-status
```

Friend:

```text
> /connect 100.81.167.95:7777
> /connections
> /who
```

## Comportamento Esperado

- O friend recebe `[DIRECT] Connected`.
- `/connections` mostra `direct_tcp`.
- `/who` mostra o peer remoto.
- DMs e mensagens seguras podem ser enviadas sem relay.

## Troubleshooting

- Confirme `tailscale status` nos dois dispositivos.
- Confirme que o IP `100.x.x.x` é do host correto.
- No Windows, permita o binário `kaya.exe` no firewall.
- Se a porta já estiver ocupada, use outra: `/listen 7788`.
