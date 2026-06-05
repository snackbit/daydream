# Milestone M1: Recorder Foundation Scratchpad

## Current Status

- Gate E complete.
- Gate F is the next implementation gate.

## Locked Decisions

- M1 targets Linux/KDE Wayland first.
- Use xdg-desktop-portal ScreenCast and PipeWire for monitor capture.
- Use FFmpeg for H.264 `yuv444p` MKV segment encoding and muxing.
- Use one composed video canvas for selected monitors.
- Use separate desktop and microphone audio streams in each segment.
- Use one active recording at a time.
- Use pause/resume as multiple MKV segment files under one recording.
- Persist scene layouts as absolute canvas pixels.
- Default frame rate is `1 FPS`.
- Default scene layout is horizontal.
- Cursor is embedded when the portal supports it.
- Monitor capture is the only M1 visual source type.
- React does not access SQLite, FFmpeg, portal, PipeWire, or Pulse directly.
- No raw private capture content is logged by default.

## Findings

- Repository workflow:
  - `AGENTS.md` makes milestone workflow optional and explicit.
  - `.agents/runbooks/planning.md` is mandatory when milestone-scoped work is
    requested.
  - `docs/milestone-template.md` defines the milestone/scratchpad artifact pair.
- Current repository:
  - Yarn 4 monorepo.
  - Desktop app: `apps/desktop`.
  - Runtime: Tauri 2, React, Vite, Tailwind, TanStack Router, Zustand.
  - Rust backend owns SQLite.
  - Diagnostics command already exists.
  - Real capture is not implemented.
- Existing SQLite v1 foundation:
  - `recordings`
  - `recording_tracks`
  - `redactions`
  - `capture_events`
  - `processing_jobs`
- Local environment evidence gathered before this milestone:
  - Session type: Wayland.
  - Desktop: KDE.
  - xdg-desktop-portal ScreenCast interface version: 5.
  - ScreenCast available source types: 7.
  - ScreenCast available cursor modes: 7.
  - PipeWire server available.
  - PulseAudio compatibility is provided by PipeWire.
  - Default sink: `evo4_stereo`.
  - Default source: `evo4_mic_stereo`.
  - Desktop audio monitor source: `evo4_stereo.monitor`.
  - Microphone source: `evo4_mic_stereo`.
- Local audio sources seen before this milestone:
  - `evo4_stereo.monitor`
  - `evo4_mic_stereo`
  - `alsa_output.usb-Audient_EVO4-00.analog-surround-40.monitor`
  - `alsa_input.usb-Audient_EVO4-00.analog-surround-40`
- Local FFmpeg evidence gathered before this milestone:
  - System `/usr/bin/ffmpeg` exposes `kmsgrab`, `pulse`, and `x11grab`.
  - The bundled `node_modules/ffmpeg-static/ffmpeg` did not expose `kmsgrab`.
  - Neither checked FFmpeg binary exposed a `pipewire` demuxer.
  - `ffmpeg -h demuxer=pipewire` returned `Unknown format 'pipewire'`.
  - M1 therefore treats FFmpeg as the encoder/muxer after Rust/PipeWire frame
    intake, not as the direct PipeWire screen capture component.

## Reference Notes

- xdg-desktop-portal ScreenCast supports:
  - multiple source selection
  - cursor modes
  - persistent restore-token behavior
  - `OpenPipeWireRemote`
- FFmpeg device docs cover Pulse input behavior and Linux capture devices.
- `kmsgrab` exists as an FFmpeg device but is intentionally not the primary M1
  path because it is privileged and less appropriate for a user-controlled
  Wayland desktop app.

## Validation Log

- `COREPACK_HOME=/tmp/corepack corepack yarn lint`: passed.
- `COREPACK_HOME=/tmp/corepack corepack yarn typecheck`: passed.
- `COREPACK_HOME=/tmp/corepack corepack yarn test`: passed.
- `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml`: passed.
- `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml`: passed after
  Gate B.
- `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml`: passed after
  Gate C.
- `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml`: passed after
  Gate C concurrent-start hardening.
- `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml`: passed after
  Gate C recorder module split.
- `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml`: passed after
  Gate D with 34 Rust tests.
- `COREPACK_HOME=/tmp/corepack corepack yarn lint`: passed after Gate D.
- `COREPACK_HOME=/tmp/corepack corepack yarn typecheck`: passed after Gate D.
- `COREPACK_HOME=/tmp/corepack corepack yarn test`: passed after Gate D.
- `COREPACK_HOME=/tmp/corepack corepack yarn workspace @daydream/desktop
  typecheck`: passed during Gate E implementation.
- `COREPACK_HOME=/tmp/corepack corepack yarn workspace @daydream/desktop
  test`: passed during Gate E implementation with 19 desktop tests, including
  Vitest Browser Mode coverage for the Capture workspace.
- `COREPACK_HOME=/tmp/corepack corepack yarn lint`: passed after Gate E.
- `COREPACK_HOME=/tmp/corepack corepack yarn typecheck`: passed after Gate E.
- `COREPACK_HOME=/tmp/corepack corepack yarn test`: passed after Gate E.
- `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml`: passed after
  Gate E with 37 Rust tests.
- `COREPACK_HOME=/tmp/corepack corepack yarn lint`: passed after Gate E shadcn
  cleanup.
- `COREPACK_HOME=/tmp/corepack corepack yarn typecheck`: passed after Gate E
  shadcn cleanup.
- `COREPACK_HOME=/tmp/corepack corepack yarn test`: passed after Gate E shadcn
  cleanup.
- `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml`: passed after
  Gate E shadcn cleanup with 37 Rust tests.
- `COREPACK_HOME=/tmp/corepack corepack yarn lint`: passed after Gate E scene
  handle cleanup.
- `COREPACK_HOME=/tmp/corepack corepack yarn typecheck`: passed after Gate E
  scene handle cleanup.
- `COREPACK_HOME=/tmp/corepack corepack yarn test`: passed after Gate E scene
  handle cleanup with 22 desktop tests.
- `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml`: passed after
  Gate E scene handle cleanup with 37 Rust tests.

## Gate Notes

- Gate A exposes camelCase recorder invoke names through Tauri command `rename`
  attributes while keeping Rust functions snake_case.
- Gate A stubs return typed serialized recorder errors with category
  `unavailable` and code `recorder_backend_not_implemented`.
- No custom Tauri capability entries were required for app-local commands in
  this scaffold; the recorder commands compile and are registered in the app
  invoke handler.
- Gate B uses `_ms` timestamp columns for new recorder tables.
- Gate B stores recorder scenes as reusable global layouts.
- Gate B minimally extends `recordings` with `active_scene_id` and
  `output_root`.
- Gate B updates `recordings.quality_label` to accept `balanced-ocr`.
- Gate B preserves existing v1 child rows while rebuilding `recordings` for the
  expanded quality-label check and new recorder metadata columns.
- Gate B makes the Rust storage module public so future recorder gates can call
  storage helpers without dead-code warnings before command wiring exists.
- Gate C implements the recorder lifecycle manager internally and registers it
  as Tauri managed state.
- Gate C wires `getRecorderStatus` to the manager and `saveRecorderScene` to
  SQLite storage.
- Gate C intentionally keeps public `startRecording`, `pauseRecording`,
  `resumeRecording`, and `stopRecording` returning
  `recorder_backend_not_implemented` until capture backends exist.
- Gate C creates segment directories and persists deterministic segment paths,
  but does not create empty MKV files.
- Gate C hardening serializes `start_internal` across the full start transition
  so concurrent starts cannot create orphaned active recording rows.
- Gate C cleanup split the recorder module into focused files for contracts,
  commands, lifecycle management, storage mapping, errors, and tests before
  Gate D adds portal and audio discovery.
- Gate D uses target-scoped Linux dependencies for ScreenCast and Pulse/PipeWire
  discovery so macOS/Windows builds keep a platform fallback path.
- Gate D adds `selectCaptureSources` as the only command allowed to show the
  ScreenCast chooser; `getCaptureSources` stays read-only.
- Gate D stores ScreenCast restore tokens in SQLite as backend-only sensitive
  metadata. Tokens are not exposed through React contracts, diagnostics, or
  logs.
- Gate D keeps `startRecording`, `pauseRecording`, `resumeRecording`, and
  `stopRecording` returning `recorder_backend_not_implemented` until the FFmpeg
  and capture-runtime gates wire real media output.
- Gate D treats portal stream size as logical compositor size, not pixel size.
  `CaptureSource.width` and `CaptureSource.height` stay null until a later
  PipeWire/capture-runtime gate can prove actual frame pixel dimensions.
- Gate E replaces the Capture placeholder with a real recorder workspace.
- Gate E uses logical portal dimensions as provisional editor pixels when
  physical capture pixels are unknown, while keeping persisted scene geometry in
  absolute canvas pixels.
- Gate E uses explicit scene saving. Drag, resize, crop, aspect, and z-order
  edits stay local until `saveRecorderScene` succeeds.
- Gate E adds Vitest Browser Mode with the Playwright provider and React Testing
  Library for critical Capture workspace UI coverage.
- Gate E leaves lifecycle commands visually present but disabled until backend
  health and saved-scene state make recording valid. Public lifecycle commands
  still remain unavailable until Gates F and G wire real media output.
- Gate E cleanup adds the shadcn components needed for the Capture workspace,
  moves the workspace to a shadcn Resizable three-panel layout, and splits the
  page into focused source, canvas, geometry, toolbar, and error components.
- Gate E scene handle cleanup adds explicit move grips, eight resize handles,
  handle-specific cursors, synthetic source placeholders, and pointer-capture
  cleanup so move/resize interactions end correctly after pointer release.
- Gate E current-scene cleanup adds `getRecorderScene`, loads the latest saved
  scene on Capture startup, changes source selection to Add-with-merge, and
  keeps source deletion as an explicit-save draft edit.

## Gate Reconciliation Log

- Gate A: reviewed Gates B-I, command contracts, and validation requirements.
  No milestone changes needed because command names and typed placeholder
  behavior already match the M1 contract.
- Gate B: reviewed Gates C-I, Storage Format, Recording Lifecycle Contract,
  Testing and Validation, and Expected Deliverables. Updated the milestone body
  to reflect concrete schema decisions: `_ms` timestamp columns,
  `recordings.active_scene_id`, `recordings.output_root`, reusable
  `recorder_scenes`, and `balanced-ocr` quality labels.
- Gate C: reviewed Gates D-I after keeping public lifecycle commands
  unavailable. No milestone changes needed because source discovery, FFmpeg,
  and UX gates still own making public capture controls usable.
- Gate C hardening: reviewed Gates D-I after serializing start transitions. No
  milestone changes needed because this preserves the existing one-active-
  recording invariant.
- Gate C module split: reviewed Gates D-I after separating recorder code into
  submodules. No milestone changes needed; the split preserves the public
  recorder command surface and creates clearer landing spots for Gate D code.
- Gate D: reviewed Gates E-I after adding explicit portal selection and audio
  discovery. Updated the milestone API to make `selectCaptureSources` a locked
  command and clarify that `getCaptureSources` is read-only cache access.
- Gate E: reviewed Gates F-I after implementing the Capture workspace and
  explicit scene-save flow. Updated the milestone body to document logical
  dimension fallback for default layouts and narrowed Gate H so it owns only the
  real FFmpeg/PipeWire-backed lifecycle connection, not the already-present
  editor shell.

## Risk Watchlist

- PipeWire stream buffers may arrive in formats that need additional conversion
  support before feeding FFmpeg.
- KDE portal restore tokens may behave differently across portal versions.
- FFmpeg packaging is not complete if M1 still depends on system FFmpeg for the
  required Pulse/encoding behavior.
- Gate F uses `DAYDREAM_FFMPEG_PATH` as an FFmpeg override and otherwise falls
  back to `ffmpeg` on `PATH`. The checked `ffmpeg-static` binary does not expose
  Pulse input, so Linux audio recording needs a capable system FFmpeg during
  development.
- Audio recording is explicit opt-in. Desktop and microphone sources are added
  and removed by the user instead of auto-selecting defaults.
- Gate F smoke validation used `/usr/bin/ffmpeg` to encode two synthetic
  rawvideo frames through stdin into `/tmp/daydream-gate-f-smoke.mkv`; `ffprobe`
  reported stream `0|h264|video|yuv444p`.
- Interactive canvas editing may need careful pointer math to stay stable on
  high-DPI displays.
- Audio defaults can change during runtime; the recorder should avoid silently
  recording the wrong source after a default-device change.
- Full-day performance and storage behavior are intentionally not proven by M1.

## Follow-Up Candidates

- Finalize installer-level FFmpeg packaging once the exact required Linux
  capabilities are proven.
- Add scissors/redaction workflow before OCR, transcription, VLM, embeddings, or
  digest work.
- Add keyboard and mouse activity instrumentation after the recorder foundation
  is stable.
- Add focused-window and open-window metadata capture behind platform adapters.
- Add processing milestones for chunk planning, frame extraction, OCR,
  transcription, VLM inference, vector indexing, and digest generation.
- Add long-duration storage and performance validation after short recorder
  smoke tests pass.
