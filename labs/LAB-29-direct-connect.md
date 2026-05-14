# LAB-29 Direct Connect

## Objetivo

Validar conexão peer-to-peer manual por TCP sem depender de UDP multicast.

## Setup

- Dois terminais ou dois dispositivos.
- KAYA buildado em ambos.
- Porta TCP disponível, exemplo `7777`.

## Comandos

Terminal A:

```text
> /listen 7777
> /listen-status
```

Terminal B:

```text
> /connect <ip-do-terminal-a>:7777
> /connections
> /who
```

## Comportamento Esperado

- Terminal A mostra listener ativo.
- Terminal B mostra conexão `direct_tcp`.
- Ambos veem o peer em `/who`.
- O painel `CONNECTIONS` lista o peer conectado.

## Troubleshooting

- `connection refused`: o listener não está ativo ou a porta está errada.
- `timed out`: firewall, IP incorreto ou rede bloqueando TCP.
- Peer duplicado: feche uma sessão com `/disconnect <peer>`.
