# Milestone M2: AI Processing Foundation Scratchpad

## Current Status

- Planning complete enough to create the initial milestone contract.
- No M2 implementation has started.
- No M2 gates are complete.

## Locked Decisions

- M2 name: AI Processing Foundation.
- M2 artifact pair:
  - `docs/milestones/m2-ai-processing-foundation.md`
  - `docs/milestones/m2-ai-processing-foundation-scratchpad.md`
- M2 scope is a full AI slice:
  - transcription
  - model management
  - AI settings
  - notification center
  - background jobs
  - activity detection
  - OCR
  - guarded VLM observation paths and playback states
  - Library playback integration
- User reaffirmed keeping the full slice rather than splitting M2 into
  separate milestone files.
- M2 should be fine-grained into sub-area goals so the broad slice remains
  reviewable during implementation.
- Processing starts automatically after recording stop.
- Automatic processing after stop is a known and user-approved exception to the
  repository's normal redaction-before-processing direction.
- Processing is resumable, chunked, and background-job based.
- Jobs support pause, cancel, retry, restart recovery, and max parallel job
  settings.
- Settings gets a dedicated `AI/LLM` tab.
- Long-running AI work appears in a header notification popover, not in main
  navigation.
- Internal AI provider calls use REST.
- Local transcription uses `whisper.cpp`.
- `whisper.cpp` C API binding is the preferred local integration.
- The local transcription endpoint is loopback-only, token-protected, and
  internal to Daydream for M2.
- Remote OpenAI-compatible endpoints are allowed after configuration.
- Local `whisper.cpp` transcription on CPU and local Tesseract OCR are the
  provider completion floor for M2 closeout.
- Hosted remote transcription and hosted VLM success are not required for M2
  closeout unless credentials and endpoint configuration are available.
- Remote transcription and VLM remain in scope as guarded provider paths:
  settings, validation, warnings, disabled-by-default behavior, privacy-safe
  errors, and adapter boundaries.
- API keys are stored in SQLite with explicit warning that this is obscured
  local storage, not OS keyring encryption.
- Daydream manages model downloads under app data.
- Default local transcription model is multilingual `small`.
- Local transcription choices include multilingual `tiny`, `base`, `small`,
  `medium`, `large-v3-turbo`, and `large-v3`.
- Model self-test is smoke-only.
- GPU/offload is detected and configurable, but CPU path is required for exit.
- Transcription runs separately for every audio track.
- Desktop and microphone transcripts remain distinguishable.
- Transcript overlay and audio lanes use matching source colors.
- Transcript playback supports word-level highlighting when timestamps are
  available.
- Library playback also gets a rolling right-side transcript view.
- VLM output is timestamped visual observations when a configured provider is
  available and its responses validate.
- M2 does not embed vLLM.
- VLM provider is OpenAI-compatible.
- OCR runs as dual OCR when VLM is configured: required local Tesseract plus
  optional guarded VLM interpretation.
- Visual processing uses conservative deltas plus periodic full-frame
  checkpoints.
- The frame similarity library must not miss subtle text changes just because
  whole-frame similarity is high.
- Abrupt visual changes need before/after context frames.
- Persist text, observations, timestamps, regions, source frame references,
  confidence where available, provider metadata, and job provenance.
- Do not persist extra cropped region images by default.
- Native activity uses cross-platform interfaces with real validation on the
  current Linux/KDE target.
- Keyboard activity default window: 30 seconds.
- Mouse activity default window: 5 seconds.
- Redaction-before-processing is deferred in M2. Processing happens
  immediately after stop, and later redaction must invalidate derived data.

## User Intent Notes

- User wants M2 to cover next steps after the recording/playback foundation.
- User wants AI and LLM capability, preferably local models.
- User wants per-track audio transcription so desktop and microphone audio do
  not get merged.
- User wants model downloads to be automatic or settings-driven.
- User wants local model choice and remote OpenAI-compatible provider choice.
- User wants internal REST use so local and remote providers share the same
  call shape.
- User wants GPU offloading surfaced as a capability when available.
- User wants a notification center for model downloads, self-tests, and
  background processing.
- User does not want notification center as a main navigation item.
- User wants OCR and VLM frame interpretation.
- User wants frame-delta and activity-aware visual processing so unchanged
  screen regions are not processed repeatedly.
- User wants activity in Library playback alongside video and audio tracks.
- User wants keyboard and mouse activity tracking to be configurable.
- User wants processing to be resumable and backgrounded.
- User explicitly chose to ignore redaction gating for M2 and process
  immediately, with redaction after processing.
- User wants transcript overlay in playback, source-colored by desktop versus
  microphone.
- User wants a rolling transcript window in the right-side detail panel.
- During M2 review, user chose:
  - keep the full slice, with finer-grained sub-area goals
  - keep automatic processing as written
  - use the local baseline as the provider completion floor

## Repository Evidence

- Repository has one prior milestone pair:
  - `docs/milestones/m1-recorder-foundation.md`
  - `docs/milestones/m1-recorder-foundation-scratchpad.md`
- `docs/milestone-template.md` defines milestone and scratchpad format.
- `.agents/runbooks/planning.md` defines milestone-scoped expectations.
- Existing Settings screen is already tabbed and uses shadcn-style components.
- Existing settings backend stores record and storage settings through Rust and
  SQLite.
- Existing SQLite schema includes:
  - `app_settings`
  - `recordings`
  - `recording_tracks`
  - `recording_segments`
  - `recorder_scenes`
  - `recorder_scene_sources`
  - `recorder_portal_restore_tokens`
  - `redactions`
  - `capture_events`
  - `processing_jobs`
- Existing `capture_events` kinds include keyboard activity and mouse activity.
- Existing `processing_jobs` includes processing concepts but needs M2 job
  kind expansion and chunk-level detail.
- Existing Library playback studio has:
  - player panel
  - track timeline
  - side panel
  - audio waveform tile support
  - local playback assets
- Current app shell has a header area suitable for adding a notification icon
  near the disabled search button.
- Existing Rust dependencies do not yet include AI, OCR, model, HTTP server,
  or activity-monitoring crates.

## External Evidence

- `whisper.cpp` provides local Whisper inference, model download tooling,
  quantization, CPU and GPU-oriented backends, C API surface, and server
  examples.
- `whisper.cpp` server documentation describes a simple HTTP server and notes
  risks around file upload handling and FFmpeg use, supporting the M2 decision
  for loopback-only binding and token enforcement.
- OpenAI transcription APIs support OpenAI-compatible transcription request
  and verbose JSON style response shapes that can carry timestamped output.
- Tesseract can provide structured OCR output with text regions and confidence
  through TSV-like output.

## Product Risks

- M2 is broad. It touches settings, backend jobs, model integration, REST,
  storage, activity monitoring, OCR, VLM, and playback UI.
- Automatic processing before redaction is privacy-weaker than the original
  repository direction.
- Remote endpoints may upload extremely sensitive media and derived text.
- Hosted remote provider validation may be unavailable without credentials or a
  configured endpoint; this should block only live remote-success validation,
  not the local M2 closeout floor.
- Word-level highlighting can imply precision that Whisper timestamps may not
  actually guarantee.
- Visual observations can overstate model certainty if not stored as
  source-linked claims.
- Frame similarity can suppress important text changes if designed too
  coarsely.
- Activity tracking can feel invasive even when only timing metadata is stored.

## Technical Risks

- `whisper.cpp` C API integration may be harder to compile and package than a
  process wrapper.
- GPU/offload support may be unreliable on the current machine until NVIDIA
  driver/runtime issues are resolved.
- Tesseract packaging and runtime discovery may be platform-specific.
- Remote provider API compatibility may vary across OpenAI-compatible vendors.
- Chunk-level retry semantics need careful idempotency to avoid duplicate
  transcript/OCR/VLM records.
- Restart recovery needs durable scheduler state and clear stale-running job
  handling.
- Loopback REST endpoint lifecycle must be tied to app lifecycle to avoid
  dangling local services.

## Stop Boundaries

- If `whisper.cpp` C API binding fails due to build or packaging portability,
  stop and document the blocker before switching to a vendored binary wrapper.
- If a remote provider stores or transmits private data in a way not visible to
  the user, stop and update the milestone contract before implementing it.
- If VLM output cannot be runtime-validated into source-linked observations,
  stop before persisting it as structured fact.
- If model download sources lack checksum or etag metadata, record the weaker
  verification behavior in provider status and the scratchpad before treating
  the download path as complete.
- If keyboard/mouse activity capture requires privileged or hidden monitoring,
  stop and record the native permission requirement before enabling it.
- If schema work changes redaction invalidation semantics, update the milestone
  body before completing that gate.
- If visual processing needs persistent cropped images for practical reasons,
  update the milestone body before storing them.

## Validation Log

- No M2 validation has run yet.

## Deferred Follow-Up Candidates

- Full scissors/redaction UI before processing.
- Daily digest generation.
- Task/todo extraction.
- Embeddings and vector search.
- Chat over recordings.
- Speaker diarization.
- OS keyring storage for remote API keys.
- Public local API endpoint for external tools.
- Full cross-platform native activity monitoring.
- Accuracy benchmark suite across languages, accents, and noisy audio.
- Installer-level packaging for model files, Tesseract, and native OCR/model
  dependencies.
