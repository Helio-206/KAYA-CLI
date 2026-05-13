# KAYA Commands

Commands are typed in the input panel. Text without `/` is sent as a public message to the current room.

The parser is registry-based: command metadata defines aliases, validation, usage strings, and generated help text. This keeps the command engine ready for autocomplete and future plugin hooks.

## Reference

| Command | Example | Description |
| --- | --- | --- |
| `/help` | `/help` | Show command summary. |
| `/who` | `/who` | List discovered peers. |
| `/rooms` | `/rooms` | List rooms known locally. |
| `/join <room>` | `/join semana-info` | Join or create a room. |
| `/room [room]` | `/room geral` | Show or switch current room. |
| `/msg <peer> <text>` | `/msg Ana teste privado` | Send a DM by callsign or node id. |
| `/status` | `/status` | Show local status and counters. |
| `/logs` | `/logs` | Toggle technical logs panel. |
| `/clear` | `/clear` | Clear visible traffic. |
| `/exit` | `/exit` | Send `LEAVE` and quit. |

Aliases:

- `/h` for `/help`
- `/j` for `/join`
- `/dm` for `/msg`
- `/q` or `/quit` for `/exit`

## Examples

```text
> /join semana-info
> alguem recebe?
> /who
> /msg KY-71AF92 teste privado
> /logs
> /exit
```
