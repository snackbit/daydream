# Milestone M1: Recorder Foundation Scratchpad

## Current Status

- Gate G complete.
- Gate H.1 PipeWire SHM capture is staged with portal live-session preflight,
  stable-ID source matching, `pipewire-rs` 0.9.2 dependency resolution,
  CPU-readable buffer extraction, DMA-BUF-required classification,
  latest-frame sampling, CPU compositor tests, and strict-live public
  `startRecording`/`resumeRecording` wiring. Manual KDE Wayland smoke
  validation is still required to prove the real local capture mode.
- Gate H.2 is partially staged beyond the original probe slice: recording can
  import supported packed single-plane DMA-BUF source frames through EGL/OpenGL,
  read them back to CPU RGBA, and feed the existing CPU compositor. GPU
  crop/scale/z-order composition and real DMA-BUF-only KDE validation remain
  open.
- Gate I gained a local Library playback validation surface for recorded MKV
  segments. Playback is local loopback streaming, thumbnails are bounded by a
  FIFO worker pool, and playback sessions are explicitly closed.

## Locked Decisions

- M1 targets Linux/KDE Wayland first.
- Use xdg-desktop-portal ScreenCast and PipeWire for monitor capture.
- Use FFmpeg with `yuv444p` rawvideo input and H.264 `yuv420p` MKV segment
  encoding and muxing.
- Do not use `libobs` directly in M1.
- Use OBS as architecture inspiration for separating platform capture adapters,
  scene composition, and output/encoding.
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
- Library playback remains local-only and session-scoped. It must not upload
  media, persist playback MP4 transcodes, or launch unbounded thumbnail FFmpeg
  processes.

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
  - FFmpeg segment writing is implemented with synthetic/controlled video frame
    input.
  - Internal PipeWire frame intake and scene composition are staged behind the
    recorder runtime boundary.
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

## OBS Inspiration Notes

- OBS separates platform-specific capture sources from scene composition and
  output/encoding. Daydream should copy that separation of concerns.
- Daydream should not copy OBS's plugin ABI, streaming services, filters,
  transitions, live production controls, or broad capture backend matrix in M1.
- The M1 architecture should introduce a Daydream-native capture runtime
  boundary before wiring the Linux Wayland PipeWire adapter into recording.
- Future Windows, macOS, and Linux X11 adapters should be able to plug into the
  same runtime boundary without changing recorder scene storage or FFmpeg
  segment metadata.

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
- `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml recorder::`:
  passed after Gate G and H.1 dependency-free preflight/compositor staging with
  46 recorder tests.
- `cargo metadata --manifest-path apps/desktop/src-tauri/Cargo.toml
  --format-version 1`: blocked by sandbox DNS when resolving the new
  `pipewire-rs` dependency.
- `cargo metadata --offline --manifest-path apps/desktop/src-tauri/Cargo.toml
  --format-version 1`: confirmed `pipewire-sys` is not available in the local
  Cargo cache, so production PipeWire stream extraction cannot be compiled in
  this environment without dependency resolution.
- `cargo metadata --manifest-path apps/desktop/src-tauri/Cargo.toml
  --format-version 1`: passed with elevated network access after adding
  `pipewire`; `pipewire` 0.10.0 resolved but failed to compile against local
  generated `libspa-sys` bindings because `SPA_ID_INVALID` was missing.
- `cargo metadata --manifest-path apps/desktop/src-tauri/Cargo.toml
  --format-version 1`: passed with elevated network access after pinning
  `pipewire` to 0.9.2.
- `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml recorder::`:
  passed after adding the internal PipeWire SHM adapter with 48 recorder tests.
- `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml --check`: found
  rustfmt drift in the new Linux capture module; `cargo fmt` was applied.
- `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml recorder::`:
  passed after strict-live public start/resume wiring with 52 recorder tests.
- `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml --check`: passed
  after strict-live public start/resume wiring.
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
- `COREPACK_HOME=/tmp/corepack corepack yarn workspace @daydream/desktop test
  recorder-workspace-store`: passed after saved-audio restore hardening with 25
  recorder workspace store tests.
- `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml
  recorder::tests::`: passed after Gate F and FFmpeg failure hardening with 19
  recorder Rust tests.
- `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml --check`:
  passed after FFmpeg failure hardening.
- `COREPACK_HOME=/tmp/corepack corepack yarn workspace @daydream/desktop
  lint`: passed after saved-audio restore hardening; ESLint still reports the
  existing `scene-layout.ts` max-lines warning.
- `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml`: passed after
  PipeWire debug log compaction and playback-server changes with 156 Rust
  tests.
- `COREPACK_HOME=/tmp/corepack corepack yarn typecheck`: passed after local
  Library playback streaming and session cleanup work.
- `COREPACK_HOME=/tmp/corepack corepack yarn lint`: passed after local Library
  playback streaming and session cleanup work; ESLint still reports existing
  max-lines warnings.
- `COREPACK_HOME=/tmp/corepack corepack yarn test`: passed with elevated
  browser-mode server binding after local Library playback streaming and
  session cleanup work; 157 JS tests passed.
- `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml
  recorder::platform::linux --lib`: passed during H.1/H.2 reconciliation with
  41 Linux recorder tests.

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
- Gate C initially kept public `startRecording`, `pauseRecording`,
  `resumeRecording`, and `stopRecording` unavailable until media output existed.
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
- Gate D kept `startRecording`, `pauseRecording`, `resumeRecording`, and
  `stopRecording` unavailable until the FFmpeg and capture-runtime gates wired
  media output.
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
- Gate E left lifecycle commands visually present but disabled until backend
  health and saved-scene state make recording valid.
- Gate E cleanup adds the shadcn components needed for the Capture workspace,
  moves the workspace to a shadcn Resizable three-panel layout, and splits the
  page into focused source, canvas, geometry, toolbar, and error components.
- Gate E scene handle cleanup adds explicit move grips, eight resize handles,
  handle-specific cursors, synthetic source placeholders, and pointer-capture
  cleanup so move/resize interactions end correctly after pointer release.
- Gate E current-scene cleanup adds `getRecorderScene`, loads the latest saved
  scene on Capture startup, changes source selection to Add-with-merge, and
  keeps source deletion as an explicit-save draft edit.
- Gate F adds FFmpeg capability probing, selected-binary diagnostics, `yuv444p`
  rawvideo input with H.264 `yuv420p` MKV segment arguments, optional Pulse
  desktop/microphone audio inputs, stream-index metadata, and process lifecycle
  management.
- Gate F public lifecycle commands now call the FFmpeg-backed recorder manager.
  This records synthetic/controlled video frames until Gate G introduces the
  capture runtime boundary and Gate H replaces that input with real PipeWire
  frame intake and composition.
- Gate F marks FFmpeg start, write, wait, and non-zero-exit failures as typed
  dependency errors, fails the active segment, marks the recording failed, and
  moves runtime status to `error`.
- Saved recorder scenes validate restored desktop and microphone audio source
  IDs against the current audio source list and required role. Unavailable or
  wrong-role IDs are dropped before the UI exposes the scene or start payload.
- Resume segment startup failures after the new segment row is created now use
  the same terminal active-recording failure path as finalize failures, so a
  later stop remains idempotent and cannot mark a failed recording completed.

## Gate Reconciliation Log

- Gate A: reviewed Gates B-J, command contracts, and validation requirements.
  No milestone changes needed because command names and typed placeholder
  behavior already match the M1 contract.
- Gate B: reviewed Gates C-J, Storage Format, Recording Lifecycle Contract,
  Testing and Validation, and Expected Deliverables. Updated the milestone body
  to reflect concrete schema decisions: `_ms` timestamp columns,
  `recordings.active_scene_id`, `recordings.output_root`, reusable
  `recorder_scenes`, and `balanced-ocr` quality labels.
- Gate C: reviewed Gates D-J after keeping public lifecycle commands
  unavailable. No milestone changes needed because source discovery, FFmpeg,
  and UX gates still own making public capture controls usable.
- Gate C hardening: reviewed Gates D-J after serializing start transitions. No
  milestone changes needed because this preserves the existing one-active-
  recording invariant.
- Gate C module split: reviewed Gates D-J after separating recorder code into
  submodules. No milestone changes needed; the split preserves the public
  recorder command surface and creates clearer landing spots for Gate D code.
- Gate D: reviewed Gates E-J after adding explicit portal selection and audio
  discovery. Updated the milestone API to make `selectCaptureSources` a locked
  command and clarify that `getCaptureSources` is read-only cache access.
- Gate E: reviewed Gates F-J after implementing the Capture workspace and
  explicit scene-save flow. Updated the milestone body to document logical
  dimension fallback for default layouts and narrowed Gate I so it owns only the
  real FFmpeg/PipeWire-backed lifecycle connection, not the already-present
  editor shell.
- Gate F: reviewed Gates G-J after implementing the FFmpeg segment writer.
  Updated the milestone body to clarify that Gate F owns synthetic/controlled
  FFmpeg input and failure handling, while later gates still own capture runtime
  integration, real PipeWire frame intake, and scene composition.
- Gate F hardening: reviewed saved audio restoration and resume failure paths.
  Updated milestone contracts so restored audio IDs must be validated against
  current source availability and role, and terminal FFmpeg failures must keep
  runtime and SQLite states consistent.
- OBS architecture reconciliation: split the next milestone work into Gate G
  capture runtime boundary and Gate H Linux Wayland PipeWire adapter and
  compositor. This keeps Daydream OBS-inspired without taking a `libobs`
  dependency or exposing OBS-style plugin/streaming complexity.
- Gate G: reviewed Gates H-J after adding the platform-neutral runtime
  boundary. Updated the milestone checklist to mark Gate G complete and split
  the old Gate H into Gate H.1 CPU-readable PipeWire SHM capture plus Gate H.2
  DMA-BUF GPU import/composition.
- Gate H split: H.1 opens a fresh portal session from `startRecording`, matches
  fresh streams to saved scene sources by stable portal ID or mapping ID, and
  feeds the existing FFmpeg `yuv444p` stdin writer. Recording no longer treats
  supported packed DMA-BUF buffers as an automatic H.1 blocker; it routes them
  through the H.2 source-frame import/readback path before CPU composition.
- Gate H.2 starts with a checked-in EGL/OpenGL DMA-BUF capability probe before
  production integration. Hardware encoding remains a deferred optimization
  because it changes the FFmpeg writer boundary and validation matrix.
- Gate H.1 hardening: CPU-readable PipeWire frame extraction now routes through
  a testable owned-copy helper. Unit coverage proves mapped bytes are copied
  before buffer return, DMA-BUF still returns `recorder_capture_dmabuf_required`,
  and debug/error output does not include frame bytes.
- Gate H.2 probe slice: added a Linux-only EGL/OpenGL capability probe using
  `khronos-egl` dynamic loading. Diagnostics now reports GPU capture health,
  DMA-BUF import support, modifier import support, and non-content driver
  strings. This does not import DMA-BUF frames or alter the FFmpeg writer path.
- Gate H.2 import/readback slice: added `LinuxGpuFrameCompositor`, which creates
  an EGL/OpenGL context, imports supported packed single-plane DMA-BUF frames as
  EGL images/textures, reads each source back to top-down CPU RGBA, and lets the
  existing CPU compositor produce the `yuv444p` FFmpeg handoff. This is not GPU
  scene composition; crop, scale, placement, and z-order still happen on CPU.
- PipeWire debug log compaction: `pipewire_capture_latest_frame_updated` and
  `pipewire_capture_scheduled_frame_composed` now emit aggregate debug events
  at most once per five-second window instead of logging per frame.
- Library playback validation surface: added a tokenized loopback playback
  server that streams FFmpeg-generated fragmented MP4 video/audio from MKV
  segments without full-file preconversion. Seeked playback uses post-input
  accurate FFmpeg seek. Timeline thumbnails are generated on demand through a
  bounded FIFO thumbnail service with two workers, duplicate request coalescing,
  and memory-only session cache.
- Library playback lifecycle hardening: React invalidates active load runs on
  unmount and closes playback sessions that resolve after route changes or
  screen unmount, preventing stale loopback sessions from surviving until app
  exit.

## Risk Watchlist

- PipeWire stream buffers may arrive in formats that need additional conversion
  support before feeding FFmpeg.
- KDE/Wayland may negotiate DMA-BUF-only screen frames. Recording now has a
  packed DMA-BUF import/readback path, but manual validation still needs to
  prove whether the real portal stream formats and modifiers work end to end.
- Planar YUV DMA-BUF import remains unsupported and must return a typed format
  error rather than producing misleading recordings.
- Hardware encoding could reduce copies after DMA-BUF import is proven, but it
  is out of M1 because M1 keeps the existing raw `yuv444p` FFmpeg handoff and
  H.264 `yuv420p` encoded output.
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
- Full-day Library playback now has bounded thumbnail FFmpeg concurrency, but
  UI-level virtualization and long-duration playback UX remain follow-up
  candidates.

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
- Virtualize or window the Library thumbnail strip after the backend queue is
  proven, so full-day recordings do not render thousands of image elements at
  once.
