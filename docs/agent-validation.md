# Validation commands and budgets

Run from the repository root unless stated otherwise. Use pinned Yarn through
Corepack. [README](../README.md) lists setup and native prerequisites.

## Choose the owning check

| Changed surface | Smallest meaningful check | Final gate when relevant |
| --- | --- | --- |
| Static docs/instructions | Review diff, links, claims and command names | No product test solely for prose. |
| Desktop logic/store/client | `corepack yarn workspace @daydream/desktop test:unit <file>` | JS gate below. |
| Interactive React workflow | `corepack yarn workspace @daydream/desktop test:browser <file>` | JS gate; desktop/narrow layout checks. |
| Shared contracts | `corepack yarn workspace @daydream/shared test` plus affected clients | Typecheck all consumers; Rust if IPC changed. |
| FFmpeg resolver | `corepack yarn workspace @daydream/ffmpeg test` | Its lint/typecheck; native integration if resolution changed. |
| Rust owner | `corepack yarn rust:test <module-or-test-filter> -- --test-threads=2` | Native gate below. |
| Migrations/redaction/deletion/processing | Relevant Rust failure/recovery/privacy scenarios | Native gate and affected browser workflow. |
| Dependencies/config | Immutable install, engine/peer review, affected checks | Relevant gates; build if bundling/packaging changed. |
| Native dev watcher/resource staging | `corepack yarn test:dev-watch` plus `rust:test --test resource_staging` | Native format/Clippy; watcher proof uses the real CLI and a synthetic runner. |

Use `COREPACK_HOME=/tmp/corepack` when the normal Corepack cache is not writable.
For constrained native checks use the authorized writable `CARGO_HOME`; do not
invent a target directory or clear caches.

```sh
COREPACK_HOME=/tmp/corepack corepack yarn workspace @daydream/desktop test:unit src/features/recorder/scene-layout.test.ts
COREPACK_HOME=/tmp/corepack corepack yarn workspace @daydream/desktop test:browser src/screens/CaptureScreen.browser.test.tsx
COREPACK_HOME=/tmp/corepack corepack yarn rust:test recorder:: -- --test-threads=2
```

## Final gates

The production user-story suite combines the JavaScript gate, native gate and
actual Tauri UI journeys:

```sh
COREPACK_HOME=/tmp/corepack CARGO_HOME=/tmp/daydream-cargo-home corepack yarn test:e2e
```

It requires the Linux build/runtime prerequisites and the native driver's GTK
Broadway/D-Bus/Playwright setup. See [user-story coverage](user-story-validation.md)
and [native runner](../apps/desktop/scripts/native-e2e/README.md). The runner
also requires matching WebKitWebDriver on `PATH` or `DAYDREAM_WEBKIT_WEBDRIVER`.
It isolates app data, uses synthetic capture, builds fresh assets and fails on missing
prerequisites; it never starts ambient devices. `yarn test:e2e:native` runs only
that native layer. Run one instance at a time.

After a coherent owned edit batch, run targeted autofix and inspect the diff:

```sh
COREPACK_HOME=/tmp/corepack corepack yarn workspace @daydream/desktop exec eslint <changed-files> --fix
```

Use `corepack yarn lint:fix` for a coordinated whole-repository batch. It rewrites
files; do not run it over another worker's unfinished changes.

Root `check` covers JavaScript lint, all workspace typechecks and unit/browser tests.
It does not compile Rust:

```sh
COREPACK_HOME=/tmp/corepack corepack yarn install --immutable
COREPACK_HOME=/tmp/corepack corepack yarn check
```

For native changes with installed Linux prerequisites:

```sh
COREPACK_HOME=/tmp/corepack corepack yarn rust:fmt
COREPACK_HOME=/tmp/corepack corepack yarn rust:clippy
COREPACK_HOME=/tmp/corepack corepack yarn rust:test -- --test-threads=2
```

`rust:fmt` checks formatting. Format owned Rust batches with
`cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml`, then inspect the diff.
`rust:clippy` checks all targets; `rust:test` uses the checked-in lockfile. Both cap
Cargo jobs at two. Direct Cargo commands use `CARGO_BUILD_JOBS=2` and `--locked`.

`test:recorder` runs focused recorder JavaScript tests and the synthetic capture/
real-writer native scenario. `test:recorder:coverage` is separate measurement;
`rust:test` is the full native gate. Synthetic capture does not prove real devices.

## Resource and runtime limits

The opt-in visual throughput check generates a 45-minute, 5120×1440 synthetic
screen recording, runs production FFmpeg extraction, adaptive sampling, bilingual
Tesseract, SQLite publication and the CPU embedding adapter, then checks search.
It requires ImageMagick (`magick`), DejaVu Sans, Tesseract with `eng` and `deu`,
and the pinned multilingual E5 weights already installed. Set
`DAYDREAM_BENCHMARK_MODELS_DIR` to a model directory when using an isolated home.
It reads model weights only; it never reads private recordings. Fixture generation
is outside the reported processing time; the debug-build processing budget is
15 minutes. Run alone so competing native workloads do not distort timing.

```sh
COREPACK_HOME=/tmp/corepack CARGO_HOME=/tmp/daydream-cargo-home corepack yarn rust:test synthetic_45_minute_local_processing_benchmark -- --ignored --nocapture --test-threads=1
```

- One coordinator schedules expensive native builds and full browser checks. Share
  two native build jobs; do not overlap Cargo builds. Reuse the target cache; do not
  build release or run `cargo clean` for normal validation.
- Vitest scripts cap workers at two. Start with focused files/projects; run full
  relevant gates once after edits settle. Repeat only for new changes/failures.
- Keep long output in `/tmp`, preserve exit status and report relevant diagnostics
  and log location. Do not dump private content.
- Browser Mode requires Playwright Chromium paired with the installed package and
  permission to launch/bind local servers. If absent, use
  `corepack yarn workspace @daydream/desktop exec playwright install chromium`.
- Rust transcription/playback tests bind temporary loopback ports. Sandbox denial
  is a prerequisite failure; obtain authorized execution or report the limitation.
- Browser tests mock IPC. Real permissions, Wayland portal, PipeWire, GStreamer,
  WebKit and packaged binaries need native/manual checks when those boundaries change.
- Retry safe read-only hardware/session diagnostics once with authorized elevation
  before treating sandbox-looking failures as host facts. Do not elevate destructive
  diagnostics or expose private content.

## Dependency verification

Read official metadata, advisories and release notes before upgrades. Check engine/
peer requirements; update coupled React/types, Vitest and Tauri packages together
where required. Generate locks through Yarn/Cargo and verify immutable/locked
checks. Scope Cargo updates by package. Treat vendored Whisper, GPU features,
models and distribution binaries as separate justified integrations.

```sh
COREPACK_HOME=/tmp/corepack corepack yarn npm audit --all --recursive --json
```

Advisory queries do not replace runtime/contract checks. State network/approval/tool
limitations and distinguish patched, unaffected, unresolved and unverified packages.
A few searched advisories cannot prove a complete all-clear.

Generated shadcn controls and their small upstream CSS helper section live in source.
The unused generator CLI is not installed just to supply CSS. Before adding new
components, verify a safe official generator/registry version, use the local
`apps/desktop/components.json`, then inspect source/CSS/manifest changes. Preserve
the third-party license; do not reintroduce the unpatched CLI graph implicitly.
