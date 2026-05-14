# KAYA Commands

Commands are typed in the input panel. Text without `/` is sent as a public message to the current room.

The parser is registry-based: command metadata defines aliases, validation, usage strings, and generated help text. This keeps the command engine ready for autocomplete and future plugin hooks.

## Reference

| Command | Example | Description |
| --- | --- | --- |
| `/help` | `/help` | Show command summary. |
| `/who` | `/who` | List discovered peers. |
| `/peers --fingerprints` | `/peers --fingerprints` | List peers with fingerprints and trust status. |
| `/rooms` | `/rooms` | List rooms known locally. |
| `/create <room>` | `/create semana-info` | Create and announce a room. |
| `/join <room>` | `/join semana-info` | Join or create a room. |
| `/leave <room>` | `/leave semana-info` | Leave a room. |
| `/current` | `/current` | Show the current room. |
| `/room <message>` | `/room sistema online` | Send text to the current room. |
| `/msg <peer> <text>` | `/msg Ana teste privado` | Send a DM by callsign or node id. |
| `/secure-msg <peer> <text>` | `/secure-msg Ana segredo` | Send an encrypted DM, creating a secure session if needed. |
| `/send <peer> <path>` | `/send Ana ./docs/PROTOCOL.md` | Offer a file to a peer. |
| `/accept-file <file_id>` | `/accept-file KF-ABCDEF123456` | Accept an incoming file. |
| `/reject-file <file_id>` | `/reject-file KF-ABCDEF123456` | Reject an incoming file. |
| `/files` | `/files` | List transfer state. |
| `/cancel-file <file_id>` | `/cancel-file KF-ABCDEF123456` | Cancel a transfer. |
| `/open-folder` | `/open-folder` | Show completed files folder. |
| `/file-info <file_id>` | `/file-info KF-ABCDEF123456` | Show metadata and progress. |
| `/presence <state>` | `/presence busy` | Set presence to `online`, `away`, `busy`, or `invisible`. |
| `/identity` | `/identity` | Show local node id, callsign, fingerprint, and public key summaries. |
| `/fingerprint` | `/fingerprint` | Show the local public fingerprint. |
| `/trust <peer>` | `/trust Ana` | Mark a known fingerprint as trusted. |
| `/untrust <peer>` | `/untrust Ana` | Return a peer to unknown trust. |
| `/block <peer>` | `/block KY-71AF92` | Block a peer from chat and DM handling. |
| `/trust-list` | `/trust-list` | Show known fingerprints and trust states. |
| `/sessions` | `/sessions` | Show secure DM session state. |
| `/close-session <peer>` | `/close-session Ana` | Close a secure DM session. |
| `/routes` | `/routes` | List mesh routing table entries. |
| `/route <target>` | `/route KY-A91C0D` | Show the best mesh route to a node id or callsign. |
| `/mesh-status` | `/mesh-status` | Show mesh relay diagnostics. |
| `/mesh-clear` | `/mesh-clear` | Clear mesh routes and seen-packet cache. |
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
- `/smsg` for `/secure-msg`
- `/send-file` for `/send`
- `/af` for `/accept-file`
- `/rf` for `/reject-file`
- `/cf` for `/cancel-file`
- `/downloads` for `/open-folder`
- `/fi` for `/file-info`
- `/p` for `/presence`
- `/id` for `/identity`
- `/fp` for `/fingerprint`
- `/mesh` for `/mesh-status`
- `/q` or `/quit` for `/exit`

## Examples

```text
> /join semana-info
> /room alguem recebe?
> /who
> /peers --fingerprints
> /trust Ana
> /secure-msg Ana segredo local
> /send Ana ./docs/PROTOCOL.md
> /files
> /sessions
> /routes
> /mesh-status
> /msg KY-71AF92 teste privado
> /presence busy
> /history semana-info
> /logs
> /exit
```
