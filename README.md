# daydream

Daydream is a planned local-first desktop recall assistant. It is meant to help
answer "what did I do today?" from private screen recordings, audio, activity
signals, window context, OCR, transcription, visual model output, and searchable
timeline annotations.

The app should run on private hardware and stay useful without requiring captured
data to leave the machine.

## Repository Status

This repository is now scaffolded as a Yarn 4 monorepo with a Tauri 2 +
React/Vite desktop app.

Implemented baseline:

- `apps/desktop`: runnable desktop shell with Home, Capture, Library,
  Diagnostics, and Settings navigation.
- `apps/desktop/src-tauri`: Rust Tauri backend with a real diagnostics command.
- `packages/ffmpeg`: development FFmpeg resolver backed by `ffmpeg-static`.
- `packages/shared`: shared diagnostics TypeScript types.
- `packages/eslint-config` and `packages/tsconfig`: shared quality and compiler
  config.

Still out of scope / not implemented:

- real screen/audio capture
- record/pause/stop commands
- SQLite schema and persistence
- scissors/redaction workflows
- OCR, Whisper, VLM, embeddings, LLM digesting
- searchable annotations and timeline playback

## Product Goal

Daydream records a workday on demand, lets the user remove sensitive spans before
processing, then builds a searchable memory of the day. It should detect and
timestamp things like:

- tasks and todos mentioned in team conversations
- "I have been working on..." status updates
- active applications and focused windows
- periods of active or idle computer use
- visible UI text from screens
- spoken desktop and microphone audio
- visual context from screen frames

The output should be a timeline that can be played back, searched, filtered, and
summarized into a daily digest with links back to source timestamps.

## Planned Capabilities

- Desktop app built with Tauri 2.
- Yarn-based JavaScript/TypeScript tooling.
- Embedded or app-managed FFmpeg so users do not need to install it manually.
- Multi-screen recording.
- Desktop audio and microphone audio recording, with streams kept distinct.
- High-quality low-rate screen recording, roughly 1 FPS by default, tuned for
  OCR accuracy and storage efficiency.
- Keyboard and mouse activity instrumentation to infer active versus idle time.
- Focused-window title, focused-window identity, and open-window inventory where
  the OS allows it.
- Pre-processing scissors/redaction so sensitive spans can be removed before
  OCR, transcription, VLM inference, embeddings, or LLM summarization.
- Local video/audio store for full-day recordings.
- Chunking, frame extraction, OCR on retained frames, Whisper transcription, VLM
  inference, and LLM digest generation.
- SQLite metadata store.
- LanceDB or Qdrant vector index behind a swappable local adapter.
- Timeline playback with searchable annotations and timestamp provenance.

## Pipeline

```text
capture
  -> local video/audio store
  -> scissors/redaction review
  -> chunk planning
  -> frame extraction
  -> OCR and UI text extraction
  -> Whisper transcription
  -> VLM inference
  -> activity and window metadata alignment
  -> SQLite metadata and vector index writes
  -> searchable timeline, annotations, tasks/todos, and daily digest
```

Capture and processing should be separate phases. The normal workflow is to
record a full day first, then process it later after the user has had a chance
to cut sensitive spans.

## Local-First Privacy Model

Daydream handles extremely sensitive data: screens, audio, OCR text,
transcripts, window titles, activity patterns, embeddings, and generated
summaries.

Baseline behavior should be local-first:

- no captured data leaves the machine by default
- raw screen/audio/text content is not logged by default
- redacted spans are excluded from all downstream processing
- deleting or cutting a span invalidates derived OCR, transcript, embedding,
  annotation, and digest data for that span
- remote model or sync features must be explicit opt-in work, not accidental
  defaults

## Target Stack

- Desktop runtime: Tauri 2.
- Language: TypeScript by default, with native modules where required.
- Package manager: Yarn 4 via Corepack.
- Capture and media: FFmpeg, currently resolved in development through
  `ffmpeg-static`.
- Speech: Whisper-style local transcription.
- OCR: local OCR engine selected for accuracy on UI text.
- Vision: local or explicitly configured VLM inference service.
- Metadata: SQLite.
- Vector search: LanceDB or Qdrant.
- UI: timeline playback, search, annotation review, digest, and scissors tools.

## Development

Prerequisites:

- Node.js 22+
- Corepack
- Rust/Cargo
- Linux Tauri system libraries that provide `webkit2gtk-4.1.pc` and
  `javascriptcoregtk-4.1.pc`

Install dependencies:

```bash
COREPACK_HOME=/tmp/corepack corepack yarn install
```

Run quality gates:

```bash
COREPACK_HOME=/tmp/corepack corepack yarn lint:fix
COREPACK_HOME=/tmp/corepack corepack yarn lint
COREPACK_HOME=/tmp/corepack corepack yarn typecheck
COREPACK_HOME=/tmp/corepack corepack yarn test
```

Run the desktop app after Linux Tauri prerequisites are installed:

```bash
COREPACK_HOME=/tmp/corepack corepack yarn dev
```

Run the frontend-only build:

```bash
COREPACK_HOME=/tmp/corepack corepack yarn workspace @daydream/desktop build
```

Run Rust tests after Linux Tauri prerequisites are installed:

```bash
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml
```

The current dev launcher builds `@daydream/ffmpeg`, resolves the downloaded
FFmpeg binary, sets `DAYDREAM_FFMPEG_PATH`, and starts `tauri dev`.

On Linux Wayland, the dev launcher also defaults
`WEBKIT_DISABLE_DMABUF_RENDERER=1`. This avoids WebKitGTK DMA-BUF protocol
failures that can show up as `Gdk-Message: Error 71 (Protocol error)
dispatching to Wayland display`, especially on NVIDIA/Wayland systems.

## Native Permission Areas

The implementation will need to handle platform-specific permissions and
capabilities for:

- screen recording
- microphone access
- desktop audio capture
- keyboard and mouse activity monitoring
- accessibility APIs
- focused-window metadata
- open-window enumeration

These integrations should be isolated behind typed adapters so OS-specific
behavior does not leak through the full application.

## Development Notes

When the repository is scaffolded, prefer small, explicit boundaries:

- capture adapters for OS and FFmpeg interaction
- storage adapters for files, SQLite, and vector index writes
- processing jobs for chunking, OCR, transcription, VLM, and LLM summarization
- domain logic for redaction eligibility, timestamp alignment, annotation
  merging, and digest construction
- UI modules for timeline, playback, search, source review, and redaction

See `AGENTS.md` for agent rules, privacy constraints, testing expectations, and
future implementation guidance.

## License

Private - All rights reserved
