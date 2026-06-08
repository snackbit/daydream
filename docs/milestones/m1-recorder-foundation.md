# Milestone M1: Recorder Foundation

## Goal

Deliver the first real recording foundation for Daydream: a local-first
Linux/KDE Wayland recorder that exposes stable domain contracts, lets the user
select and arrange monitor sources, records desktop and microphone audio as
separate tracks, writes pause/resume segments to local storage, persists
metadata in SQLite, and surfaces backend health in the app.

## Summary

- This milestone turns the Capture placeholder into a working recorder
  workspace backed by Rust/Tauri commands.
- The capture backend uses xdg-desktop-portal and PipeWire for screen capture,
  then FFmpeg for encoding and muxing MKV segment files.
- The first target platform is Linux/KDE Wayland on the current development
  machine, with platform-specific code isolated behind recorder interfaces.
- Processing work remains out of scope: OCR, transcription, VLM inference,
  indexing, source-linked recall timeline, searchable annotations, and daily
  digest generation are not implemented in M1.
- M1 now includes a local Library playback validation surface for recorded MKV
  segments so recorder output can be inspected without full-file MP4
  preconversion.
- This is the right boundary because downstream recall features need durable,
  privacy-conscious recordings with timestamps, source layout metadata, and
  separate audio tracks before they can process anything accurately.

## Inputs

- `AGENTS.md`
- `README.md`
- `docs/milestone-template.md`
- `.agents/runbooks/planning.md`
- Current Tauri app under `apps/desktop`
- Current Rust SQLite storage under `apps/desktop/src-tauri/src/storage.rs`
- Current diagnostics command under
  `apps/desktop/src-tauri/src/diagnostics.rs`
- Current development FFmpeg resolver under `packages/ffmpeg`
- xdg-desktop-portal ScreenCast interface reference:
  <https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.ScreenCast.html>
- FFmpeg device reference:
  <https://www.ffmpeg.org/ffmpeg-devices.html>
- OBS Studio backend design and capture-source architecture as implementation
  inspiration, not as a direct dependency:
  <https://docs.obsproject.com/backend-design>

## Locked Decisions

- Milestone artifacts:
  - `docs/milestones/m1-recorder-foundation.md`
  - `docs/milestones/m1-recorder-foundation-scratchpad.md`
- The first implementation target is Linux/KDE Wayland.
- The primary capture architecture is portal bridge plus FFmpeg:
  - xdg-desktop-portal ScreenCast obtains monitor streams.
  - PipeWire carries screen frames from the portal.
  - Rust owns frame intake and scene composition.
  - FFmpeg owns encoding and muxing.
- Daydream will not embed or link `libobs` in M1. The recorder uses
  OBS-inspired separation between platform capture adapters, scene composition,
  and output/encoding, but keeps those boundaries native to the Daydream Rust
  backend.
- Platform capture adapters must stay behind an internal capture runtime
  boundary so Linux Wayland, future Linux X11, macOS, and Windows capture paths
  can be added without rewriting scene storage, recorder lifecycle, or FFmpeg
  segment output.
- Privileged `kmsgrab` is not the primary path for M1.
- The app requests persistent portal permission where available with
  `persist_mode=2`, stores restore tokens as sensitive local metadata, and
  falls back to the KDE chooser when a restore token is missing, expired, or
  rejected.
- The first visual source type is monitor capture only.
- Multi-monitor support is required from the first implementation.
- Multiple selected monitors are composed into one video canvas.
- The default canvas layout is automatic horizontal placement.
- Scene layout storage uses canonical absolute canvas pixels. The UI may show
  percentages, but persisted source-of-truth values are absolute pixels.
- The scene editor follows an OBS Studio-style interaction model:
  - move sources on a canvas
  - resize sources
  - crop sources
  - preserve-aspect-ratio toggle
  - source list with selection state
- Cursor capture is embedded in the video stream when the portal supports it.
- Recording output is one MKV file per segment.
- A recording can contain multiple segment files.
- Pause/resume creates a new segment file under the same recording.
- Each segment contains separate streams for:
  - composed screen video
  - desktop audio
  - microphone audio
- The default frame rate is `1 FPS`.
- The default video encoding profile is balanced H.264 with `yuv420p` output.
  The internal FFmpeg rawvideo handoff remains `yuv444p`.
- The app records video, desktop audio, and microphone audio from the first
  backend implementation when the user explicitly adds those sources.
- Desktop audio and microphone audio are optional, user-controlled sources.
  They are not auto-selected on startup.
- Restored scene audio source IDs must be validated against current audio
  sources and required roles before the UI exposes them or recording starts.
- The app supports one active recording at a time.
- The recorder API exposes these commands to the frontend:
  - `getRecorderStatus`
  - `getCaptureSources`
  - `getRecorderScene`
  - `selectCaptureSources`
  - `getAudioSources`
  - `saveRecorderScene`
  - `startRecording`
  - `pauseRecording`
  - `resumeRecording`
  - `stopRecording`
- `getCaptureSources` is read-only and returns cached selected monitor sources.
- `getRecorderScene` returns the latest saved recorder scene, or `null` when no
  scene has been saved.
- `selectCaptureSources` is the explicit portal-selection command. Passing
  `{ forceChooser: true }` bypasses restore-token reuse and shows the portal
  chooser. It must not leak portal-specific session details into React state.
- Default recording files live under
  `<app_data_dir>/recordings/{recordingId}/segments`.
- SQLite schema M1 uses segments plus scenes:
  - recordings remain the parent entity
  - recording segments represent pause/resume output files
  - scene records represent capture canvas configuration
  - scene source records represent selected monitor placement and crop state
- SQLite access remains Rust-owned. React uses typed Tauri commands only.
- Diagnostics must include recorder backend health:
  - portal availability
  - ScreenCast version and source/cursor capabilities
  - PipeWire availability
  - selected FFmpeg path and capability summary
  - default desktop audio source ID
  - default microphone source ID
- The FFmpeg binary remains configurable.
- Gate F uses `DAYDREAM_FFMPEG_PATH` as an FFmpeg override and otherwise falls
  back to `ffmpeg` on `PATH`. The checked `ffmpeg-static` build lacks Pulse
  input support and is not enough for Linux audio recording.
- M1 does not ship a final installer-level FFmpeg packaging solution. It must
  leave a concrete follow-up if the development bridge is still required.
- Library playback for M1 is a local validation and inspection tool, not the
  final recall timeline. It streams browser-playable media from local MKV
  segments through a tokenized loopback playback session.
- Playback thumbnails are generated on demand through a bounded FIFO worker
  pool and cached only in memory for the active playback session. Thumbnail
  generation must not fan out unbounded FFmpeg processes for full-day
  recordings.
- Closing a playback session must remove session-scoped thumbnail cache entries
  and reject stale queued thumbnail jobs.
- The app must not log raw frames, raw audio, OCR-like text, transcripts,
  focused window titles, or private recording content.
- Recording must be visible and user-controlled. No hidden background capture is
  introduced in M1.

## Success Criteria

The milestone is successful when:

- The Capture screen can discover available monitor and audio sources.
- The user can arrange selected monitors in an interactive canvas editor.
- The user can start, pause, resume, and stop one active recording.
- The app writes MKV segment files under the recording directory.
- The Library can inspect available recordings through local streaming playback
  without permanently transcoding playback MP4 files.
- Each recorded segment contains a composed video stream plus separate desktop
  and microphone audio streams when those inputs are available.
- Recording, segment, track, scene, and source metadata are persisted in SQLite.
- Diagnostics shows whether portal, PipeWire, FFmpeg, desktop audio, and mic
  inputs are usable.
- Permission denial, unavailable portal, unavailable audio, unavailable FFmpeg,
  and invalid lifecycle transitions return typed user-facing errors.
- No captured private content is logged by default.
- The final validation gate records real command results and manual smoke-test
  notes in the scratchpad.

## Scope

- Recorder domain contracts and TypeScript/Rust IPC payloads.
- Rust recorder state machine for idle, selecting, recording, paused, stopping,
  error, and unavailable states.
- SQLite migration and storage helpers for scenes and segments.
- Linux/KDE Wayland ScreenCast portal integration.
- PipeWire frame intake for portal monitor streams.
- Compositor for selected monitor streams into one video canvas.
- FFmpeg argument building and process lifecycle for MKV segment output.
- Pulse/PipeWire audio source discovery and FFmpeg audio input mapping.
- Capture screen UI with status, controls, source selectors, and scene editor.
- Local Library playback session support for inspecting recorder output.
- Recorder diagnostics in the existing Diagnostics screen.
- Focused Rust and TypeScript tests for contracts, lifecycle, storage, FFmpeg
  args, layout math, and error states.
- Manual end-to-end recording validation on the development machine.

## Non-Goals

- OCR, UI text extraction, Whisper transcription, VLM inference, embeddings,
  LLM digest generation, and task/todo extraction.
- Source-linked recall timeline, searchable annotations, and daily digest UI.
- Permanent playback MP4 generation or remote playback services.
- Scissors/redaction implementation.
- Keyboard/mouse activity instrumentation.
- Focused-window or open-window metadata capture.
- Cross-platform capture backends for Windows or macOS.
- Direct `libobs` integration, OBS plugin ABI support, live streaming services,
  filters, transitions, and broadcast-production features.
- Final application installer packaging.
- Final vendored FFmpeg distribution policy.
- Privileged `kmsgrab` capture fallback.
- Remote model or sync features.

## Definitions and Ownership Rules

- Recording: a user-controlled capture session that may contain one or more
  segment files.
- Segment: a continuous MKV output file produced between start and stop, or
  between resume and pause/stop.
- Scene: the canvas configuration used to compose selected monitor streams.
- Scene source: one selected monitor within a scene, including source identity,
  placement, size, crop, z-order, and aspect-ratio behavior.
- Capture source: a monitor source exposed through the ScreenCast portal.
- Capture adapter: a platform-specific backend that turns native capture
  permissions and streams into recorder runtime frame inputs.
- Capture runtime: the platform-neutral Rust boundary between selected capture
  sources, timestamped frame delivery, scene composition, and segment output.
- Captured frame: a timestamped video buffer plus format metadata. Raw frame
  contents are private media and must not be logged.
- Scene compositor: the deterministic Rust stage that maps captured frames into
  the saved recorder scene canvas before FFmpeg encoding.
- Audio source: a Pulse/PipeWire input source or sink monitor selectable for
  desktop or microphone capture.
- Recorder status: the typed state returned to React. It is the source of truth
  for buttons, warnings, and current recording metadata.
- Backend health: capability and availability data surfaced through
  Diagnostics. It must not contain private screen/audio content.
- React owns presentation and transient edit state.
- Rust owns native capture, FFmpeg lifecycle, SQLite writes, filesystem paths,
  and recorder lifecycle validation.
- SQLite owns durable recorder metadata.
- MKV files own encoded media payloads.

## Architectural Invariants

- Native integrations stay behind recorder modules; UI code does not talk to
  portal, PipeWire, Pulse, FFmpeg, or SQLite directly.
- Platform capture adapters feed the capture runtime; they do not write
  SQLite, own scene layout, or invoke FFmpeg directly.
- Scene composition consumes capture runtime frames and saved scene geometry; it
  stays independent of the Linux PipeWire adapter.
- FFmpeg segment writing consumes composed frames and optional audio sources; it
  stays independent of portal and PipeWire session details.
- All Tauri command payloads are serializable, typed, and mirrored in shared
  TypeScript contracts where practical.
- Every recording segment has a stable recording ID, segment ID, file path,
  start timestamp, and eventual end timestamp or failure state.
- Recording and segment timestamps use a monotonic runtime source for duration
  math and wall-clock timestamps for user display.
- Segment file names are deterministic within a recording directory.
- FFmpeg child processes are killed or awaited on lifecycle transitions and app
  shutdown.
- Pause, stop, and failure paths finalize or mark segment records so processing
  jobs can later avoid partial or failed files.
- Terminal FFmpeg failures clear active runtime state, preserve the last error,
  and leave SQLite recording and segment state failed. This includes resumed
  segment startup failures after the new segment row has been created.
- Source layout math is deterministic and testable without a live portal.
- The app may degrade when desktop audio or microphone input is unavailable,
  but it must surface that state before recording starts.
- Recorder errors use typed categories so the UI can distinguish permission,
  dependency, lifecycle, filesystem, audio, video, and encoder failures.

## Public API and Config Changes

The recorder command surface is stable for M1:

- `getRecorderStatus() -> RecorderStatus`
- `getCaptureSources() -> CaptureSourceList`
- `getRecorderScene() -> RecorderScene | null`
- `selectCaptureSources(input?: SelectCaptureSourcesInput) -> CaptureSourceList`
- `getAudioSources() -> AudioSourceList`
- `saveRecorderScene(input: SaveRecorderSceneInput) -> RecorderScene`
- `startRecording(input: StartRecordingInput) -> RecorderStatus`
- `pauseRecording(input: PauseRecordingInput) -> RecorderStatus`
- `resumeRecording(input: ResumeRecordingInput) -> RecorderStatus`
- `stopRecording(input: StopRecordingInput) -> RecorderStatus`

The API must include these domain payloads:

- `RecorderStatus`
- `RecorderState`
- `RecorderError`
- `CaptureSource`
- `CaptureSourceSelection`
- `SelectCaptureSourcesInput`
- `AudioSource`
- `AudioSourceRole`
- `RecorderScene`
- `RecorderSceneSource`
- `CanvasRect`
- `CropRect`
- `StartRecordingInput`
- `PauseRecordingInput`
- `ResumeRecordingInput`
- `StopRecordingInput`
- `RecorderSegment`
- `RecorderBackendHealth`

Internal Rust interfaces added by M1 must include platform-neutral equivalents
for capture runtime, capture adapter, capture session, runtime source handle,
captured frame, captured frame format, and scene compositor concepts. These are
not Tauri IPC contracts.

Config behavior:

- `DAYDREAM_FFMPEG_PATH` remains accepted in development.
- The selected FFmpeg path is visible in Diagnostics.
- Audio source overrides are stored as source IDs, not display labels.
- Restored audio source IDs are dropped when the current source list no longer
  contains that ID for the saved role.
- Scene layout values are stored as absolute pixels.
- The default scene is generated from selected monitor dimensions with
  horizontal layout and no crop. Physical pixel dimensions are preferred when
  known; logical portal dimensions are used as the Gate E editor fallback until
  PipeWire frame intake proves actual frame pixels.

## Native Permission Requirements

- The Linux ScreenCast portal is the user-facing permission boundary for monitor
  capture.
- The implementation requests monitor sources with multi-selection enabled.
- The implementation requests cursor embedding when available.
- The implementation requests restore-token behavior where KDE portal supports
  it.
- A denied or closed portal chooser returns a permission error and does not
  create a recording row.
- An invalid restore token falls back to the chooser without crashing.
- Pulse/PipeWire audio source discovery reads local server metadata.
- Missing desktop or microphone input is surfaced as degraded backend health and
  as a preflight warning in the Capture screen.

## Privacy and Deletion Contract

- M1 records sensitive media, so all logs and diagnostics must avoid raw screen,
  audio, OCR-like text, transcripts, prompts, and private content.
- Diagnostics may report source IDs, source roles, capability flags, file paths,
  and dependency versions.
- Recording files are local-only under the app data directory by default.
- M1 does not implement deletion or redaction, but schema and segment metadata
  must be compatible with later deletion and redaction eligibility work.
- Failed or partial segment records must be distinguishable from valid segments
  so later processing does not consume unsafe files.
- Portal restore tokens are stored as sensitive local metadata and are not shown
  in the UI.

## Storage Format

SQLite migration M1 adds explicit segment and scene storage.

Expected tables:

- `recording_segments`
  - `id`
  - `recording_id`
  - `segment_index`
  - `file_path`
  - `started_at_ms`
  - `ended_at_ms`
  - `duration_ms`
  - `state`
  - `video_stream_index`
  - `desktop_audio_stream_index`
  - `microphone_audio_stream_index`
  - `ffmpeg_exit_code`
  - `error_code`
  - `created_at_ms`
  - `updated_at_ms`
- `recorder_scenes`
  - `id`
  - `name`
  - `canvas_width`
  - `canvas_height`
  - `layout_mode`
  - `created_at_ms`
  - `updated_at_ms`
- `recorder_scene_sources`
  - `id`
  - `scene_id`
  - `source_id`
  - `source_kind`
  - `display_name`
  - `x`
  - `y`
  - `width`
  - `height`
  - `crop_left`
  - `crop_top`
  - `crop_right`
  - `crop_bottom`
  - `z_index`
  - `preserve_aspect_ratio`
  - `created_at_ms`
  - `updated_at_ms`

Existing table changes:

- `recordings` includes `active_scene_id` and `output_root`.
- `recordings.quality_label` accepts `balanced-ocr`.
- `recorder_scenes` are reusable scene layouts.
- `recordings.active_scene_id` links a recording to the active scene.
- `recording_tracks` may be reused for stream-level records when that avoids
  duplication, but segment-level stream indexes must remain queryable.

Migration stop boundary:

- Schema edits and embedded migrations happen in the same implementation gate.
- Migration tests must run with `cargo test --manifest-path
  apps/desktop/src-tauri/Cargo.toml`.
- No migration may depend on React or frontend code.

## Recording Lifecycle Contract

- `idle -> recording` starts a new recording and segment.
- `recording -> paused` finalizes the current segment.
- `paused -> recording` starts a new segment for the same recording.
- `recording -> idle` stops and finalizes the current recording.
- `paused -> idle` stops the recording without creating a new segment.
- `error -> idle` is allowed only after cleanup has run and the status reports
  the last error.
- Starting while recording or paused returns a lifecycle error.
- Pausing while idle returns a lifecycle error.
- Resuming while idle returns a lifecycle error.
- Stopping while idle is idempotent and returns idle status.
- App shutdown while recording or paused attempts cleanup and marks any active
  segment with a terminal state.

## Gate A: Recorder Contracts and Command Surface

- [x] Add shared recorder domain types for status, state, errors, sources,
  scenes, layout, audio roles, segments, and command inputs.
- [x] Add Rust recorder domain types that serialize to the shared frontend
  contract shape.
- [x] Register recorder Tauri commands with typed placeholder implementations
  that compile and return realistic unavailable or idle responses.
- [x] Verify Tauri capability entries required for the recorder commands.
- [x] Add TypeScript command client helpers for every recorder command.
- [x] Add tests for command payload typing, lifecycle error shape, and frontend
  client behavior.

Exit Criteria:

- All M1 command names exist and are callable through the Tauri command layer.
- No real native capture is started in Gate A.
- `COREPACK_HOME=/tmp/corepack corepack yarn typecheck` passes.
- `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` passes.

## Gate B: SQLite Recorder Schema and Storage Helpers

- [x] Add embedded SQLite migration for `recording_segments`,
  `recorder_scenes`, and `recorder_scene_sources`.
- [x] Update storage initialization so existing databases migrate cleanly to the
  M1 schema.
- [x] Add storage helpers for creating recordings, scenes, scene sources, and
  recording segments.
- [x] Add storage helpers for finalizing, failing, and listing segment records.
- [x] Add constraints that prevent duplicate segment indexes per recording.
- [x] Add tests for fresh database migration, existing database migration,
  segment lifecycle writes, scene persistence, and constraint behavior.

Exit Criteria:

- SQLite v2 schema is explicit and tested.
- React still has no direct SQLite access.
- `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` passes.

## Gate C: Recorder State Machine and File Layout

- [x] Implement the Rust recorder manager as Tauri managed state.
- [x] Enforce one active recording at a time.
- [x] Implement lifecycle transitions for start, pause, resume, stop, error,
  cleanup, and app shutdown.
- [x] Generate recording directories under
  `<app_data_dir>/recordings/{recordingId}/segments`.
- [x] Generate deterministic segment file names.
- [x] Wire lifecycle transitions to SQLite recording and segment writes.
- [x] Add tests for valid transitions, invalid transitions, cleanup behavior,
  segment numbering, and file path generation.

Exit Criteria:

- Recorder status reflects the current lifecycle state and active segment.
- Invalid lifecycle calls return typed errors.
- Segment metadata remains durable across pause/resume/stop.

## Gate D: Portal and Audio Source Discovery

- [x] Implement Linux ScreenCast portal capability probing.
- [x] Implement monitor source selection with multiple-source support.
- [x] Implement restore-token storage and chooser fallback behavior.
- [x] Normalize portal monitor streams into recorder capture source records.
- [x] Implement Pulse/PipeWire audio source discovery.
- [x] Map default sink monitor to the desktop audio role.
- [x] Map default source to the microphone audio role.
- [x] Surface portal, PipeWire, desktop audio, and microphone availability in
  recorder status and Diagnostics.
- [x] Add tests for capability parsing, restore-token storage, audio source
  role mapping, and unavailable lifecycle command behavior.

Exit Criteria:

- The Capture screen can list monitor and audio candidates without starting a
  recording.
- Denied portal permission and missing audio inputs return typed degraded or
  error states.
- Portal session details do not leak into React state beyond stable source IDs
  and display-safe metadata.

## Gate E: Scene Layout and Interactive Editor

- [x] Add deterministic default horizontal layout generation for selected
  monitor dimensions.
- [x] Implement canvas layout math for move, resize, crop, z-order, and
  preserve-aspect-ratio behavior.
- [x] Persist scene edits through `saveRecorderScene`.
- [x] Replace the Capture placeholder with a recorder workspace.
- [x] Add status header, start/pause/resume/stop controls, source list, audio
  selectors, FPS display, quality preset display, and scene canvas.
- [x] Implement OBS Studio-style pointer interactions for moving and resizing
  scene sources.
- [x] Add crop controls that persist absolute pixel crop values.
- [x] Add tests for layout math, scene persistence client behavior, and critical
  UI states.

Exit Criteria:

- A user can create and edit a multi-monitor scene before recording.
- Persisted scene values are absolute pixels.
- UI state derives button availability from `RecorderStatus`.

## Gate F: FFmpeg Segment Writer and Audio Tracks

- [x] Implement FFmpeg capability probing for the selected binary.
- [x] Implement FFmpeg argument builder for one MKV segment with `yuv444p`
  rawvideo input, H.264 `yuv420p` output, `1 FPS`, and optional
  desktop/microphone audio.
- [x] Implement configurable FFmpeg path selection with `DAYDREAM_FFMPEG_PATH`
  support.
- [x] Start and stop FFmpeg child processes from the recorder manager.
- [x] Pipe or feed synthetic or controlled video frames into FFmpeg without
  logging frame content.
- [x] Attach explicitly selected desktop and microphone audio inputs as separate
  optional FFmpeg streams.
- [x] Persist segment stream indexes and FFmpeg exit status in SQLite.
- [x] Add tests for FFmpeg args, selected binary reporting, process failure
  handling, and stream-index metadata.

Exit Criteria:

- The selected FFmpeg binary and capability summary are visible in Diagnostics.
- A synthetic or controlled video input can produce a valid MKV segment in test
  or manual validation.
- FFmpeg failures mark the active segment failed and return typed recorder
  errors.
- Real capture runtime integration, PipeWire frame intake, and scene
  composition remain owned by Gates G and H.

## Gate G: Capture Runtime Boundary

- [x] Add internal Rust capture runtime contracts for selected source runtime
  handles, capture sessions, timestamped frames, frame formats, and adapter
  lifecycle.
- [x] Preserve portal PipeWire node metadata as backend-only runtime data
  without exposing portal session internals to React.
- [x] Add a platform-neutral scene compositor input model that maps
  `RecorderScene` sources to runtime frame inputs.
- [x] Define the initial supported frame path as CPU-readable raw video frames
  that can be converted to the FFmpeg writer's `yuv444p` input.
- [x] Return typed video or dependency errors for unsupported frame formats,
  missing runtime sources, adapter startup failures, and adapter cleanup
  failures.
- [x] Add tests for source-to-runtime mapping, frame format classification,
  compositor input planning, adapter lifecycle cleanup, and privacy-safe error
  messages.

Exit Criteria:

- Recorder lifecycle can depend on platform-neutral capture runtime contracts
  instead of Linux PipeWire details.
- Unsupported frame formats fail before writing misleading complete segments.
- Raw frame contents are not logged, serialized into errors, or exposed through
  React state.

## Gate H.1: PipeWire SHM Capture and CPU Compositor

- [x] Add Linux-only `pipewire-rs` integration for portal monitor streams opened
  from a fresh `startRecording` portal session.
- [x] Match fresh portal streams to saved scene sources by stable portal stream
  ID or mapping ID. Missing matches fail before recording rows are created.
- [x] Negotiate and ingest CPU-readable PipeWire buffers for `yuv444p`,
  RGBx/RGBA/BGRx/BGRA, and XRGB/ARGB-equivalent raw formats.
- [x] Classify DMA-BUF buffers separately from CPU-readable buffers so
  preview/CPU-only paths return typed errors while recording can route
  supported packed DMA-BUF frames through Gate H.2.
- [x] Copy only sampled CPU-readable frames into owned recorder runtime frames
  and return PipeWire buffers promptly.
- [x] Composite selected monitor streams into the configured scene canvas using
  deterministic CPU bilinear crop, scale, placement, and z-order behavior.
- [x] Pace composition with latest-frame sampling at the configured FPS and feed
  composed `yuv444p` frames to the existing FFmpeg segment writer.
- [x] Add tests for source matching, SHM format mapping, DMA-BUF-required
  errors, frame sampling, CPU compositor math, FFmpeg handoff, cleanup, and
  privacy-safe error/debug output.

Exit Criteria:

- Public `startRecording` uses the real PipeWire capture path, not synthetic
  video, for non-fake scenes.
- CPU-readable streams use the SHM/CPU compositor path; supported packed
  DMA-BUF streams route through H.2 source-frame import/readback before the CPU
  compositor. Preview and unsupported DMA-BUF formats return typed errors.
- Frame content is not logged or exposed through React state.

## Gate H.2: DMA-BUF GPU Import and Composition

- [x] Add a checked-in Linux-only EGL/OpenGL capability probe for DMA-BUF import
  and modifier support.
- [x] Use the probe to lock thin native dependencies for EGL/OpenGL DMA-BUF
  work before production integration.
- [x] Import supported packed single-plane DMA-BUF frames into EGL/OpenGL
  textures.
- [x] Defer full GPU crop, scale, placement, and z-order composition out of M1
  because M1 validates DMA-BUF import/readback while retaining the existing
  CPU compositor and FFmpeg writer boundary.
- [x] Read imported DMA-BUF source frames back to CPU RGBA, then feed the
  existing CPU compositor and `yuv444p` FFmpeg rawvideo handoff while keeping
  encoded output at `yuv420p`.
- [x] Add privacy-safe capture timing diagnostics that expose CPU-readable vs
  DMA-BUF frame counts and DMA-BUF readback counts through
  `getRecorderPerformanceStats`.
- [ ] Validate that DMA-BUF-only portal streams on KDE/Wayland produce the same
  MKV stream shape as the SHM path, or record a concrete native blocker when
  the local portal does not negotiate DMA-BUF-only frames.
- [x] Keep hardware encoding out of M1 and document it as a later optimization
  because it changes the writer boundary and validation matrix.

Exit Criteria:

- Supported packed DMA-BUF source frames can be imported, read back to CPU, and
  composed through the CPU compositor into the same MKV stream shape as the SHM
  path.
- Real DMA-BUF-only portal behavior is validated on KDE/Wayland, or a concrete
  native blocker is recorded in the scratchpad.
- Unsupported DMA-BUF formats or modifiers return typed recorder errors before
  misleading complete segments are written.
- GPU diagnostics and errors remain privacy-safe.

## Gate I: End-to-End Recorder UX and Diagnostics

- [ ] Connect Capture controls to real FFmpeg/PipeWire-backed recorder
  lifecycle commands once Gates G and H make recording available.
- [ ] Show backend readiness, degraded audio states, dependency failures, and
  lifecycle errors in user-facing UI.
- [ ] Keep recording controls disabled or warning-gated when required sources
  are unavailable.
- [x] Extend Diagnostics with recorder backend health rows.
- [ ] Add status refresh behavior that stays accurate during start, pause,
  resume, stop, and failure.
- [x] Add local Library playback sessions for recorder-output inspection with
  tokenized loopback URLs, realtime FFmpeg streaming, bounded thumbnail
  generation, and session cleanup.
- [ ] Add UI tests for recorder status rendering, disabled controls, lifecycle
  actions, and diagnostics rendering.

Exit Criteria:

- The user can operate the recorder from the app without terminal commands.
- A recorded Library item can be inspected locally without unbounded thumbnail
  generation or durable playback transcode files.
- Diagnostics distinguishes unavailable, degraded, and available backend states.
- The UI does not display portal restore tokens or private captured content.

## Gate J: Validation and Launch Readiness

- [ ] Run JavaScript lint fix.
- [ ] Run JavaScript lint.
- [ ] Run TypeScript typecheck.
- [ ] Run JavaScript tests.
- [ ] Run Rust tests.
- [ ] Run the desktop frontend build.
- [ ] Run a manual Tauri dev smoke test.
- [ ] Record a short multi-monitor segment with desktop and microphone audio
  enabled when available.
- [ ] Pause and resume once, then stop.
- [ ] Verify the recording directory contains multiple MKV segment files.
- [ ] Verify each complete segment has the expected video, desktop audio, and
  microphone streams with `ffprobe`.
- [ ] Verify SQLite contains matching recording, segment, scene, and track
  metadata.
- [ ] Verify Diagnostics reports portal, PipeWire, FFmpeg, desktop audio, and
  microphone status.
- [ ] Verify no raw private capture content appears in logs.
- [ ] Update the scratchpad validation log and gate reconciliation log with
  command results, manual smoke notes, residual risks, and follow-up
  candidates.

Exit Criteria:

- Required automated gates pass or documented environmental blockers are
  recorded in the scratchpad.
- Manual recording smoke test succeeds on Linux/KDE Wayland or a specific
  native blocker is documented with evidence.
- M1 residual risks are narrow enough for the next milestone to proceed.

## Testing and Validation

Required validation:

- `COREPACK_HOME=/tmp/corepack corepack yarn lint:fix`
- `COREPACK_HOME=/tmp/corepack corepack yarn lint`
- `COREPACK_HOME=/tmp/corepack corepack yarn typecheck`
- `COREPACK_HOME=/tmp/corepack corepack yarn test`
- `COREPACK_HOME=/tmp/corepack corepack yarn workspace @daydream/desktop build`
- `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml`

Manual validation:

- `COREPACK_HOME=/tmp/corepack corepack yarn dev`
- Start a short recording from the Capture screen.
- Pause, resume, and stop.
- Inspect MKV streams with the selected FFmpeg/FFprobe tooling.
- Inspect Diagnostics for recorder backend health.
- Inspect the app logs for absence of private raw capture content.

Suggested native diagnostics:

- `gdbus introspect --session --dest org.freedesktop.portal.Desktop --object-path /org/freedesktop/portal/desktop`
- `pactl info`
- `pactl list sources short`
- `pw-cli --version`
- `<selected-ffmpeg> -devices`
- `<selected-ffmpeg> -formats`

## Dependencies and Risks

- Portal behavior can vary between desktop environments and portal backend
  versions.
- KDE restore-token behavior must be validated against real chooser flows.
- PipeWire frame formats may require support for SHM, DMA-BUF, or format
  conversion paths.
- The bundled `ffmpeg-static` binary may lack required Linux capture or audio
  capabilities, so M1 may need the system FFmpeg as a development bridge.
- Multi-monitor composition can increase CPU and memory pressure even at 1 FPS.
- Desktop audio source names and defaults can change while the app is running.
- Microphone permission and Pulse/PipeWire routing failures must not silently
  produce misleading complete recordings.
- Full-day storage behavior is not validated in M1; short smoke recordings are
  the M1 completion bar.
- Full-day Library playback thumbnail generation is bounded by the playback
  server worker pool, but long-duration UX performance remains a follow-up risk.

## Assumptions

- The current development environment is Linux/KDE Wayland.
- xdg-desktop-portal and xdg-desktop-portal-kde are installed.
- PipeWire and pipewire-pulse are installed.
- Pulse-compatible FFmpeg audio input is available through the selected FFmpeg
  binary.
- The first implementation can target monitor streams only.
- Absolute pixel layout values are sufficient for later OCR frame extraction and
  timeline provenance.
- The current SQLite app-data database may already exist and must migrate
  forward without manual deletion.

## Expected Deliverables

- Recorder domain contract types in Rust and TypeScript.
- Registered Tauri recorder commands.
- SQLite v2 recorder schema and tests.
- Recorder manager with lifecycle enforcement.
- Portal monitor source discovery and restore-token fallback.
- Audio source discovery and desktop/mic role mapping.
- Scene layout storage and interactive Capture screen editor.
- FFmpeg segment writer with `yuv444p` rawvideo input and H.264 `yuv420p` MKV
  output.
- OBS-inspired internal capture runtime boundary for platform capture adapters,
  frame format metadata, scene compositor inputs, and FFmpeg output handoff.
- PipeWire compositor path for selected monitor streams.
- Local Library playback server for inspecting recorder segments with bounded
  thumbnail generation.
- Recorder backend health in Diagnostics.
- Automated tests for the critical contracts and lifecycle behavior.
- Scratchpad validation log for final command results and manual smoke tests.
