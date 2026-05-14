# KAYA File Transfer

Phase 4 adds offline peer-to-peer file transfer over the existing UDP multicast runtime. It does not add mesh routing or store-and-forward relays.

## Flow

```text
FILE_OFFER
  -> FILE_ACCEPT or FILE_REJECT
  -> FILE_CHUNK or FILE_CHUNK_ENCRYPTED
  -> FILE_CHUNK_ACK
  -> FILE_TRANSFER_COMPLETE
```

Cancellation and failures use:

- `FILE_TRANSFER_CANCEL`
- `FILE_TRANSFER_ERROR`

## Storage Layout

KAYA creates:

```text
~/.kaya/files/
├── incoming/
├── completed/
├── temp/
└── metadata/
```

Received files are written only under `completed/`, never to the sender's original path. Metadata is persisted as JSON so `/files` and `/file-info <file_id>` survive restarts.

## Metadata

Each offer carries:

- `file_id`
- `file_name`
- `file_size`
- optional `mime_type`
- final `sha256`
- `chunk_size`
- `total_chunks`
- sender node id and callsign
- creation timestamp
- dangerous-extension warning
- encrypted/unencrypted marker

Names are validated to reject absolute paths, separators, NUL bytes, `.`/`..`, and path traversal.

## Chunking

Default chunk size is `64 KiB`. Each chunk carries:

- `file_id`
- `chunk_index`
- `total_chunks`
- chunk SHA-256
- payload
- timestamp

The receiver validates every chunk hash and then validates the final file SHA-256 before saving the completed payload.

## Security

- Files are never accepted automatically.
- Blocked peers are rejected before file routing.
- Incoming offers show callsign, fingerprint, size, and encrypted state.
- If `file_transfer.accept_from_unknown = false`, offers from unknown peers are rejected.
- If a secure DM session already exists, chunks are sent as `FILE_CHUNK_ENCRYPTED` using the session-derived file-transfer context.
- Without a secure session, chunks may be sent as plaintext and are marked `unencrypted`.

## Config

```toml
[file_transfer]
enabled = true
max_file_size_mb = 50
chunk_size_kb = 64
accept_from_unknown = true
download_dir = "~/.kaya/files/completed"
```

When `download_dir` is omitted, KAYA uses `~/.kaya/files/completed`.
