# Milestone M1: Recorder Foundation Scratchpad

## Current Status

- Gate A complete.
- Gate B is the next implementation gate.

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

## Gate Notes

- Gate A exposes camelCase recorder invoke names through Tauri command `rename`
  attributes while keeping Rust functions snake_case.
- Gate A stubs return typed serialized recorder errors with category
  `unavailable` and code `recorder_backend_not_implemented`.
- No custom Tauri capability entries were required for app-local commands in
  this scaffold; the recorder commands compile and are registered in the app
  invoke handler.

## Risk Watchlist

- PipeWire stream buffers may arrive in formats that need additional conversion
  support before feeding FFmpeg.
- KDE portal restore tokens may behave differently across portal versions.
- FFmpeg packaging is not complete if M1 still depends on system FFmpeg for the
  required Pulse/encoding behavior.
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
