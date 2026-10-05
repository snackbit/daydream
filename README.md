# Daydream

Daydream is a local-first desktop recall assistant for answering one question:
**what did I do today?**

It records screen, audio, activity, and window context on hardware you control,
then turns that evidence into a timeline you can review and process locally.

![Daydream recording studio showing playback, redacted spans, audio waveforms, and activity lanes](docs/desktop-screenshot1.png)

> [!WARNING]
> Daydream is a Linux-first developer alpha. There are no stable packaged
> releases yet, and several product areas are still under active development.

## What works today

- Multi-monitor capture through the Linux desktop portal and PipeWire.
- A scene editor for arranging selected displays on one recording canvas.
- Separate desktop and microphone audio streams in local MKV segments.
- Explicit start, pause, resume, and stop controls.
- Local Library playback with thumbnails, waveforms, track lanes, and source
  timestamps.
- Timeline redactions that exclude or invalidate overlapping derived results.
- Protected-window rules that can pause capture or create automatic redactions.
- Mouse, keyboard, frame-change, and window activity lanes without recording
  typed text or pointer coordinates.
- Resumable processing jobs for transcription, OCR, visual observations, and
  activity analysis.
- Local transcription through vendored `whisper.cpp`.
- Full-frame OCR documents from local Tesseract (default) or explicitly configured
  Vision OCR, with readable text, source-linked revision history and details.
- Optional OpenAI-compatible remote transcription and vision endpoints.
- Rust-owned SQLite storage with embedded migrations.

## Recorded by Daydream

The demonstration below was recorded by Daydream itself. The animation is
silent; the original desktop and microphone tracks were not added to this
repository.

![Daydream recording its own Diagnostics, Settings, Notifications, and Capture screens](docs/daydream-demo.gif)

## Privacy model

Daydream handles screen recordings, audio, OCR, transcripts, window titles,
activity patterns, and generated observations. The default architecture keeps
that data on the local machine.

- Captured content is not uploaded by default.
- Remote AI providers require explicit configuration.
- API keys are stored through the backend rather than exposed to React.
- Raw frames, audio, OCR text, transcripts, prompts, and window titles are not
  written to normal logs.
- Redacted ranges are excluded from later processing and hidden or invalidated
  in derived results.
- Input activity records timing and state, not key values, typed text, pointer
  coordinates, or clicked content.
- Vision OCR sends original full-frame images only when selected and configured;
  it is independent of screen-observation processing. Existing OCR data is
  reconstructed locally without an automatic remote request.
- Recording state stays visible and user-controlled.

The codebase treats deletion and redaction as data-model concerns, not visual
filters. Work is still ongoing to make physical media deletion and every
downstream cleanup path equally complete.

## Current limits

- Development and real capture testing currently focus on Linux, especially KDE
  Plasma on Wayland. Windows and macOS capture adapters are not implemented.
- The searchable day index, semantic search, daily digest, task extraction, and
  cited question answering are planned work.
- There is no stable installer, automatic updater, or supported release channel.
- Some Linux capabilities depend on desktop portal, PipeWire, GStreamer,
  Tesseract, input-device permissions, and compositor support.
- Full GPU composition and hardware encoding remain future work.
- Optional remote model endpoints can receive private content when the user
  enables and configures them.

## Architecture

Daydream is a Yarn 4 monorepo:

- `apps/desktop`: Tauri 2, React, Vite, Tailwind, shadcn components, TanStack
  Router, and Zustand.
- `apps/desktop/src-tauri`: Rust capture, playback, processing, settings,
  diagnostics, and SQLite ownership.
- `packages/shared`: TypeScript contracts shared across the desktop boundary.
- `packages/ffmpeg`: development FFmpeg resolution through `ffmpeg-static`.
- `packages/eslint-config` and `packages/tsconfig`: shared project tooling.

The Linux recorder uses xdg-desktop-portal and PipeWire for screen sources,
GStreamer for segmented media writing, and separate desktop and microphone
audio tracks. Processing runs after capture in deterministic, resumable chunks.

```text
capture
  -> local MKV segments and activity metadata
  -> redaction-aware chunk planning
  -> transcription, OCR, activity, and optional vision processing
  -> SQLite-backed timeline results
  -> local playback and review
```

React does not access SQLite, native capture APIs, media writers, or model
runtimes directly. Typed Tauri commands own those boundaries.

OCR recognizes full snapshots at the configured frame-recognition sampling rate.
Exact-image reuse reduces repeated engine calls while retaining sampled occurrences;
whitespace-only text grouping preserves source timestamps and splits at redactions.
Settings select the OCR engine separately from screen observations. Retry keeps a
job's pinned configuration; explicit OCR reprocessing uses current settings without
rerunning audio. See [OCR document behavior](docs/ocr-documents-design.md) for schema,
legacy reconstruction, provider failure and privacy details.

## Development

### Prerequisites

- Node.js 22.12+ on the Node 22 line, or Node 24+ (Node 24 LTS recommended)
- Corepack
- Rust and Cargo 1.95 or newer (the locked `rusqlite_migration` dependency requires it)
- Linux Tauri development libraries for WebKitGTK 4.1
- PipeWire and xdg-desktop-portal
- GStreamer development headers and the base, good, bad, ugly, and PulseAudio
  plugin sets
- CMake, Clang, and libclang for the vendored `whisper.cpp` build
- Tesseract for local OCR

On Debian or Ubuntu, the native development packages include:

```bash
sudo apt install \
  libwebkit2gtk-4.1-dev \
  build-essential \
  curl \
  wget \
  file \
  libxdo-dev \
  libssl-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  libpipewire-0.3-dev \
  libpulse-dev \
  libgstreamer1.0-dev \
  libgstreamer-plugins-base1.0-dev \
  libgstreamer-plugins-bad1.0-dev \
  gstreamer1.0-plugins-base \
  gstreamer1.0-plugins-good \
  gstreamer1.0-plugins-bad \
  gstreamer1.0-plugins-ugly \
  gstreamer1.0-libav \
  gstreamer1.0-tools \
  gstreamer1.0-pulseaudio \
  cmake \
  clang \
  libclang-dev \
  tesseract-ocr
```

Package names vary by distribution.

### Install and run

```bash
COREPACK_HOME=/tmp/corepack corepack yarn install --immutable
COREPACK_HOME=/tmp/corepack corepack yarn dev
```

On Linux Wayland, the development launcher sets
`WEBKIT_DISABLE_DMABUF_RENDERER=1` by default to avoid known WebKitGTK DMA-BUF
protocol failures.

### Quality gates

```bash
COREPACK_HOME=/tmp/corepack corepack yarn check
COREPACK_HOME=/tmp/corepack corepack yarn rust:fmt
CARGO_BUILD_JOBS=2 COREPACK_HOME=/tmp/corepack corepack yarn rust:clippy
CARGO_BUILD_JOBS=2 COREPACK_HOME=/tmp/corepack corepack yarn rust:test -- --test-threads=2
```

`check` covers JavaScript lint, typechecks and tests; native checks are separate.
See the [validation map](docs/agent-validation.md) for focused commands,
formatting, worker budgets and Playwright/loopback prerequisites.

## Project documents

Detailed design decisions and implementation history live under
[`docs/milestones`](docs/milestones):

- [Recorder foundation](docs/milestones/m1-recorder-foundation.md)
- [AI processing foundation](docs/milestones/m2-ai-processing-foundation.md)
- [Track timeline editor](docs/milestones/m2.1-track-timeline-editor.md)
- [Day index MVP plan](docs/milestones/m3-day-index-mvp.md)
- [Protected-window recording rules](docs/protected-window-recording-rules.md)
- [Full-frame OCR documents](docs/ocr-documents-design.md)

## Contributing and security

Read [CONTRIBUTING.md](CONTRIBUTING.md) before opening a change.

Do not attach real recordings, screenshots, transcripts, OCR output, window
titles, or unredacted logs to public issues. Report security problems privately
as described in [SECURITY.md](SECURITY.md).

## License

Daydream is licensed under the
[GNU Affero General Public License v3.0 or later](LICENSE).

Copyright © 2026 Felix Faust.

Third-party code, tools, models, and runtime components retain their own
licenses. See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
