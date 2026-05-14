# KAYA Commands

Commands are typed in the input panel. Text without `/` is sent as a public message to the current room.

The parser is registry-based: command metadata defines aliases, validation, usage strings, and generated help text. This keeps the command engine ready for autocomplete and future plugin hooks.

## Reference

| Command | Example | Description |
| --- | --- | --- |
| `/help` | `/help` | Show command summary. |
| `/who` | `/who` | List discovered peers. |
| `/rooms` | `/rooms` | List rooms known locally. |
| `/create <room>` | `/create semana-info` | Create and announce a room. |
| `/join <room>` | `/join semana-info` | Join or create a room. |
| `/leave <room>` | `/leave semana-info` | Leave a room. |
| `/current` | `/current` | Show the current room. |
| `/room <message>` | `/room sistema online` | Send text to the current room. |
| `/msg <peer> <text>` | `/msg Ana teste privado` | Send a DM by callsign or node id. |
| `/presence <state>` | `/presence busy` | Set presence to `online`, `away`, `busy`, or `invisible`. |
| `/history [room]` | `/history semana-info` | Show local room history. |
| `/dm-history <peer>` | `/dm-history Helio` | Show local DM history with a peer. |
| `/status` | `/status` | Show local status and counters. |
| `/logs` | `/logs` | Toggle technical logs panel. |
| `/clear` | `/clear` | Clear visible traffic. |
| `/exit` | `/exit` | Send `LEAVE` and quit. |

Aliases:

- `/h` for `/help`
- `/j` for `/join`
- `/part` for `/leave`
- `/dm` for `/msg`
- `/p` for `/presence`
- `/q` or `/quit` for `/exit`

## Examples

```text
> /join semana-info
> /room alguem recebe?
> /who
> /msg KY-71AF92 teste privado
> /presence busy
> /history semana-info
> /logs
> /exit
```
