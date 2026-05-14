# Contributing to KAYA CLI

Thanks for contributing to KAYA CLI.

## Principles

- keep changes focused and reviewable
- preserve the local-first design
- do not introduce protocol or runtime changes casually
- document user-visible behavior and honest limitations
- prefer small, testable steps over broad rewrites

## Development Setup

```bash
cargo build
cargo test
```

For a release-style validation pass:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test
cargo build --release
```

## Before Opening a Pull Request

- explain the problem clearly
- explain why the chosen change is the smallest reasonable fix
- include tests when behavior changes
- update docs when commands, UX, packaging, or operational guidance changes
- avoid unrelated refactors in the same PR

## Scope Guidance

Please open an issue before proposing:

- protocol redesigns
- transport changes
- major UI rewrites
- security-model changes
- large dependency additions

## Commit and PR Expectations

- use clear commit messages
- describe how the change was validated
- mention any known trade-offs or follow-up work
- keep PRs easy to review in one sitting when possible

## Demo and Docs

If your change affects product positioning or public presentation, update the relevant files in `docs/` or `presentation/` so the repository stays GitHub-ready.

## Security

If you believe you found a security issue, do not open a public exploit report first. Follow the process in [SECURITY.md](SECURITY.md).
