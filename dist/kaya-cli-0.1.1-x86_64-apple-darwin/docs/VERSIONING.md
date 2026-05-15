# KAYA Versioning

## Current versions

- `kaya-cli 0.1.1`
- `kaya-sdk 0.1.1`
- `kaya-protocol 0.1.1`

## SemVer policy

KAYA uses semantic versioning.

- patch: bug fixes and non-breaking packaging or docs updates
- minor: backwards-compatible API additions
- major: breaking public API or protocol changes

## API stability

Stable public surfaces in `0.1.1`:

- CLI flags and binary name `kaya`
- `kaya-sdk` client-facing API
- `kaya-protocol` packet schema for `0.1.x`

Internal crates are not guaranteed stable for direct third-party consumption unless documented as public.

## Breaking changes

A change is considered breaking when it alters:

- `kaya-sdk` method signatures or exported types
- binary install path assumptions
- packet compatibility within the same declared protocol series

## Compatibility matrix

| CLI | SDK | Protocol | Compatibility |
| --- | --- | --- | --- |
| 0.1.x | 0.1.x | 0.1.x | compatible |
| 0.2.x | 0.1.x | 0.2.x | review required |
| 1.x | 0.x | 0.x/1.x | breaking expected |

## Release guidance

If the protocol changes in a wire-incompatible way, bump both the protocol crate and the release notes explicitly.