# Contributing to Daydream

Daydream processes highly sensitive personal data. Contributions must preserve
local execution, explicit user control, and real deletion or invalidation of
derived data.

## Before opening a change

- Discuss large product or architecture changes in an issue first.
- Keep platform-specific capture code behind typed boundaries.
- Keep SQLite access in the Rust/Tauri backend.
- Do not add a remote data path without explicit user configuration and clear
  UI copy.
- Do not add real recordings, screenshots, transcripts, OCR output, window
  titles, prompts, API keys, or embeddings as fixtures.
- Use synthetic data for tests and examples.

## Development setup

Install the prerequisites listed in [README.md](README.md), then install the
workspace through the pinned Yarn version:

```bash
COREPACK_HOME=/tmp/corepack corepack yarn install
```

Run the desktop app with:

```bash
COREPACK_HOME=/tmp/corepack corepack yarn dev
```

## Checks

Run the relevant focused tests while working. Before submitting a change, run:

```bash
COREPACK_HOME=/tmp/corepack corepack yarn lint:fix
COREPACK_HOME=/tmp/corepack corepack yarn check
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml
```

Rust tests that exercise local transcription endpoints need permission to bind
temporary loopback ports.

## Pull requests

- Explain the user workflow or failure mode the change protects.
- Include tests with behavior changes.
- State which commands you ran.
- Call out manual checks that were skipped.
- Document native permissions, storage formats, or model/tool tradeoffs when
  they change.

By contributing, you agree that your contribution is licensed under
`AGPL-3.0-or-later`.
