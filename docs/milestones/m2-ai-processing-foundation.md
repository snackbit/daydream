# Milestone M2: AI Processing Foundation

## Goal

Deliver Daydream's first AI processing foundation: after a recording stops, the
app starts resumable local-first processing that transcribes each audio track
separately, detects user and screen activity, extracts OCR text, supports
guarded VLM visual observations when configured, and shows source-linked results
in Library playback.

## Summary

- M2 turns recorded media into the first useful recall layer.
- The first implementation target is a full vertical slice, not only model
  setup: settings, model management, processing jobs, local model calls,
  guarded remote provider paths, transcript playback, activity lanes, OCR
  regions, and VLM observation states all need to work together.
- The processing system is job-based, chunked, resumable, cancellable, and
  visible through a persistent notification surface.
- Transcription uses `whisper.cpp` locally by default, exposed through an
  internal OpenAI-compatible REST boundary.
- OCR and VLM are dual inputs: Tesseract provides the required local structured
  OCR baseline, while a configured OpenAI-compatible vision endpoint may produce
  visual observations and may help interpret OCR output.
- M2 intentionally defers redaction-before-processing. Processing starts after
  recording stop. This is a user-approved exception to the repository's normal
  redaction-before-processing direction. Later redaction work must invalidate
  derived data for removed spans.

## Inputs

- `AGENTS.md`
- `README.md`
- `docs/milestone-template.md`
- `.agents/runbooks/planning.md`
- `docs/milestones/m1-recorder-foundation.md`
- `docs/milestones/m1-recorder-foundation-scratchpad.md`
- Existing settings implementation under `apps/desktop/src/screens/SettingsScreen.tsx`
- Existing Rust settings command under `apps/desktop/src-tauri/src/settings.rs`
- Existing SQLite migration owner under `apps/desktop/src-tauri/src/storage/schema.rs`
- Existing Library playback studio under
  `apps/desktop/src/features/library/components/studio/`
- Existing recorder output contract: MKV segments with screen video, desktop
  audio, and microphone audio streams kept separate when selected
- `whisper.cpp` repository and server documentation:
  <https://github.com/ggml-org/whisper.cpp>
- OpenAI-compatible transcription endpoint shape:
  <https://platform.openai.com/docs/api-reference/audio/createTranscription>
- Tesseract OCR documentation and TSV output behavior:
  <https://tesseract-ocr.github.io/>

## Locked Decisions

- Milestone artifacts:
  - `docs/milestones/m2-ai-processing-foundation.md`
  - `docs/milestones/m2-ai-processing-foundation-scratchpad.md`
- M2 ships a full AI slice: transcription, activity detection, OCR, guarded VLM
  observation paths, settings, jobs, and playback integration.
- Processing starts automatically after a recording stops.
- Automatic processing after stop is a user-approved M2 exception to the
  repository's normal redaction-before-processing target.
- Processing is chunked and resumable so app shutdown and restart can continue
  unfinished work.
- Background processing jobs support pause, cancel, retry, and restart
  recovery.
- The Settings screen gains an `AI/LLM` tab.
- The app gains a persistent header notification popover for model downloads,
  self-tests, processing jobs, failures, pause/cancel/retry actions, and
  restart-recovery status.
- The notification surface must not become a main navigation item.
- Internal AI processing calls use REST consistently.
- Local transcription is implemented with `whisper.cpp`.
- The preferred `whisper.cpp` integration is Rust plus C API binding.
- If `whisper.cpp` C API integration becomes blocked by build portability,
  packaging, or cross-platform linker complexity, the gate must stop and record
  the blocker before switching to a vendored binary wrapper.
- Daydream exposes a local OpenAI-compatible transcription endpoint for the
  internal app to consume.
- The local transcription endpoint binds only to loopback, uses an ephemeral
  port, requires a per-session token, and is not exposed as an external user
  API in M2.
- Remote OpenAI-compatible transcription and VLM endpoints are allowed when the
  user configures them.
- Remote provider configuration is a normal provider option, not only a hidden
  developer feature.
- Local `whisper.cpp` transcription on CPU and local Tesseract OCR are the
  required provider floor for milestone closeout.
- Successful hosted remote transcription or hosted VLM calls are not required
  for milestone closeout unless credentials and endpoint configuration are
  available during validation.
- Remote provider paths are still in scope for settings, validation, warnings,
  disabled-by-default behavior, privacy-safe errors, and adapter boundaries.
- Remote upload paths must be visibly labeled because audio, frames, OCR text,
  transcripts, and model prompts are sensitive private data.
- Sensitive settings such as remote API keys are stored as AES-256-GCM
  encrypted base64 envelopes in the unified settings table, using a Linux
  Secret Service-backed Daydream app key. macOS and Windows encrypted secret
  storage return typed not-implemented states until later cross-platform work.
- Model downloads are app-managed.
- Local model files live under the app data directory.
- The default local transcription model is multilingual `small`, not
  English-only `small.en`.
- Local transcription model choices include multilingual `tiny`, `base`,
  `small`, `medium`, `large-v3-turbo`, and `large-v3`.
- English-only model variants may be offered as optional advanced choices, but
  they are not the default.
- Model self-test is smoke-only in M2: prove the model loads and returns a
  transcription result. Accuracy fixtures are deferred.
- GPU acceleration is detected and configurable, but CPU transcription is the
  required validation path for milestone exit.
- Cargo package defaults remain CPU-safe so normal CI and fresh contributor
  builds do not require GPU SDKs. Developer checkouts may opt into local
  `whisper.cpp` GPU builds through an ignored `apps/desktop/tauri-dev.local`
  file or `DAYDREAM_TAURI_FEATURES`; this checkout enables `whisper-vulkan`
  locally and keeps `whisper-cuda` opt-in until the CUDA toolkit is installed.
- Transcription runs separately on every audio track.
- Desktop audio and microphone transcripts remain distinguishable in storage
  and UI.
- Transcript playback supports word-level highlighting when the provider
  returns word timestamps.
- Transcript overlay text is color-coded by source track.
- Audio source lanes in the timeline use colors that match transcript overlay
  colors.
- The right side of the Library playback studio shows a rolling transcript view
  synchronized to playback.
- VLM output in M2 is timestamped visual observations, not a final digest,
  chat assistant, or embedding-backed search system.
- The first VLM provider is a configurable OpenAI-compatible vision endpoint.
- M2 does not embed vLLM itself.
- OCR uses dual processing when VLM is configured: Tesseract plus VLM.
- Tesseract produces structured OCR candidates with regions and confidence.
- VLM may interpret regions, produce observations, and help explain or correct
  low-confidence OCR, but Tesseract raw OCR results remain stored.
- Visual processing uses conservative frame deltas: changed regions plus
  periodic full-frame checkpoints.
- Visual processing also tracks already processed frames or regions by
  similarity, while guarding against subtle text changes that can score as very
  similar.
- Abrupt visual changes must consider before/after frames so context is not
  lost.
- Persist derived text, observations, timestamps, source frame references,
  region boxes, confidence where available, provider metadata, and processing
  job provenance.
- Do not persist extra cropped region images by default.
- M2 creates cross-platform native activity interfaces but only requires real
  validation on the current Linux/KDE Wayland development target.
- Unsupported platform adapters return typed unavailable states.
- Activity detection combines frame deltas with keyboard and mouse activity.
- Keyboard and mouse activity capture is configurable in settings.
- Default keyboard activity window is 30 seconds.
- Default mouse activity window is 5 seconds.
- Library playback gains an activity gauge or lane similar to video and audio
  tracks.
- M2 explicitly overrides the repo's earlier pre-processing redaction ideal:
  redaction-before-processing is deferred. The required M2 privacy behavior is
  clear labeling, local-first defaults, no private-content logs, and derived
  data invalidation support for later redaction.

## Success Criteria

The milestone is successful when:

### M2 Closeout Minimum

- Local `whisper.cpp` transcription can download, load, smoke-test, and
  transcribe on CPU.
- Local Tesseract OCR produces timestamped structured OCR regions.
- A stopped recording automatically queues durable processing jobs.
- The app can restart and resume unfinished processing chunks without losing
  progress.
- Processing jobs can be paused, cancelled, retried, and surfaced through the
  persistent header notification popover.
- Library playback shows transcript, activity, OCR, and VLM disabled,
  unavailable, empty, error, or populated states as appropriate.
- Derived-data invalidation helpers exist for later redaction by recording ID
  and time range.
- Remote provider upload remains disabled unless explicitly configured.
- Private raw media, transcript content, OCR text, VLM prompts, window titles,
  and model responses are not logged by default.
- Required TypeScript and Rust quality gates pass.

### Sub-Area Goals

- AI settings and notifications: the user can configure all M2 local AI
  behavior, guarded remote providers, processing concurrency, model downloads,
  and activity settings from the `AI/LLM` settings tab.
- Local model and transcription: the local transcription endpoint is
  loopback-only, token-protected, and consumed internally through the same
  OpenAI-compatible REST shape used for remote providers.
- Processing scheduler: stopped recordings queue processing, unfinished chunks
  recover after restart, and failed or cancelled chunks do not create misleading
  derived records.
- Transcript playback: each selected audio stream produces its own transcript
  with segment timestamps and word timestamps when available.
- Activity detection: frame deltas plus available keyboard/mouse timing produce
  active, idle, and unknown intervals.
- Visual processing: local OCR is required; VLM observations are produced only
  when a configured provider is available and validated.
- Playback integration: Library playback shows source-colored transcript
  overlay, transcript sidebar, matching audio lane colors, clickable transcript
  segments, activity lane, OCR regions, and VLM observation states.
- Privacy and deletion readiness: remote endpoint usage is explicit, private
  content is absent from default logs, and future redaction can remove or
  invalidate overlapping derived records.

The milestone is not successful if any required local baseline behavior is
missing, even if a remote provider path works.

## User Workflows and Test Matrix

| Workflow | State combinations | Automated test layer | Manual check | Privacy/logging risk |
| --- | --- | --- | --- | --- |
| Configure AI provider | local model missing/downloaded/self-test failed/remote endpoint configured | Browser Mode, TS unit, Rust settings tests | Verify labels for local vs remote providers | API keys and remote upload disclosure |
| Download model | queued/running/succeeded/failed/cancelled | Rust job/model tests, Browser Mode | Verify persistent notification and retry | Download URL/path only, no media |
| Stop recording and auto-process | one track/two tracks/no audio/failed segment/restart during job | Rust processing tests, Browser Mode | Verify processing resumes after app restart | Media paths and job metadata only in logs |
| Transcript playback | desktop only/mic only/both tracks/no word timestamps/word timestamps | Browser Mode, TS timeline tests | Verify overlay color and seek behavior | Transcript text must not be logged |
| Activity playback | frame-only/input-only/combined/unknown permission | Rust activity tests, Browser Mode | Verify lane readability at desktop and narrow widths | Input activity is sensitive metadata |
| OCR/VLM observations | unchanged frames/subtle text changes/abrupt changes/VLM disabled/VLM invalid response/remote VLM failure | Rust visual pipeline tests | Inspect region links and failure recovery | OCR text, frames, prompts, responses |
| Job controls | pause/cancel/retry/app restart/max parallel jobs | Rust job scheduler tests, Browser Mode | Verify controls from notification popover | Error messages must not include private text |

## Scope

- AI settings schema, validation, persistence, UI, and frontend store updates.
- Managed model cache for local transcription models.
- `whisper.cpp` C API integration and local OpenAI-compatible transcription
  REST endpoint.
- Remote OpenAI-compatible transcription provider configuration.
- Guarded OpenAI-compatible VLM endpoint configuration.
- Background processing job scheduler, chunk planner, resume metadata, and
  notification state.
- Audio demuxing and per-track transcription for recording segments.
- Transcript segment and word storage with source track identity.
- Tesseract OCR integration for selected frame regions.
- VLM visual observation adapter, validation, and playback states for selected
  frame regions and periodic checkpoints.
- Frame similarity and delta-region planning for visual processing.
- Keyboard/mouse activity adapter interfaces and Linux/KDE validation path.
- Playback studio transcript overlay, rolling transcript panel, activity lane,
  OCR display, VLM observation states, and matching track colors.
- Focused tests for storage, processing, settings, REST security, UI workflows,
  and privacy-safe logging.

## Non-Goals

- Final redaction/scissors workflow.
- Blocking all processing until user redaction review.
- Daily digest generation.
- Task/todo extraction.
- Embeddings and vector search.
- Chat interface over recordings.
- Embedding or managing vLLM as a bundled runtime.
- Full cross-platform native keyboard/mouse monitoring implementation.
- Final installer-level packaging for every model and OCR dependency.
- Public external API for other tools to call Daydream's local endpoint.
- Speaker diarization.
- Perfect word-level alignment.
- Accuracy benchmarking across languages and noisy audio.
- Persistent cropped-image storage for processed regions.

## Definitions and Ownership Rules

- AI provider: a local or remote service used for transcription, OCR/VLM, or
  future language processing.
- Local transcription endpoint: the loopback HTTP endpoint hosted by Daydream
  for `whisper.cpp` transcription and consumed internally through REST.
- Remote endpoint: an OpenAI-compatible endpoint configured by the user and
  allowed to receive private captured content for the selected operation.
- Model asset: a local model file managed by Daydream under app data.
- Processing job: durable unit of background work for a recording.
- Processing chunk: resumable unit inside a job, usually bounded by recording
  segment, time range, source stream, or frame interval.
- Transcript segment: timestamped text span produced from one audio source.
- Transcript word: optional word-level timestamp entry attached to a transcript
  segment.
- OCR region: timestamped screen region with text candidates and confidence.
- VLM observation: timestamped model-produced visual claim linked to source
  frame and region provenance.
- Activity sample: keyboard, mouse, frame-delta, or merged activity metadata.
- Frame delta: a visual change estimate between sampled frames.
- Frame similarity library: local processing metadata used to avoid repeatedly
  processing unchanged or near-identical regions.
- React owns UI state, presentation, and command invocation.
- Rust owns SQLite, model files, native adapters, local REST endpoint lifecycle,
  processing jobs, filesystem paths, media demuxing, and model runner
  integration.
- SQLite owns durable metadata and derived AI records.
- Media files own original encoded screen and audio payloads.

## Architectural Invariants

- React never queries SQLite directly.
- React never invokes `whisper.cpp`, Tesseract, FFmpeg, native input monitors,
  or VLM processes directly.
- All AI calls from Daydream processing code go through typed provider
  adapters, even when the selected provider is REST.
- The local transcription endpoint must bind to loopback only.
- The local transcription endpoint must require a per-session token for every
  request.
- The local transcription endpoint must shut down cleanly with app lifecycle.
- Provider switching must not change stored transcript, OCR, or observation
  provenance semantics.
- Every derived record must identify its recording, source type, time range,
  provider, and processing job or chunk.
- Every transcript record must preserve audio source identity.
- Word timestamps are optional provider output, not a hard requirement for all
  providers.
- Timestamp math is relative to the recording timeline for UI and storage.
- Segment-local timestamps must be converted to recording offsets before
  persistence.
- Processing must skip failed or incomplete recording segments.
- Processing must not write derived results for cancelled chunks.
- Retrying a chunk must either replace prior failed partial output or write a
  new attempt that cleanly supersedes it.
- No logs may contain raw frames, raw audio, transcripts, OCR text, VLM prompts,
  VLM responses, API keys, or model responses by default.
- Remote providers must be visibly labeled before they can receive captured
  content.
- Platform-specific activity adapters stay behind typed interfaces.
- Unsupported native adapters must return typed unavailable states rather than
  fake success.

## Public API / Interface / Config Changes

M2 adds these Tauri command surfaces. Names may be adjusted to the repository's
existing camelCase command convention during implementation, but the behavior
must remain equivalent:

- `getAppSettings() -> AppSettingsState`
- `saveAppSettings(input: SaveAppSettingsInput) -> AppSettingsState`
- `resetAppSettings() -> AppSettingsState`
- `listAiModels() -> AiModelAssetList`
- `downloadAiModel(input: DownloadAiModelInput) -> Notification`
- `deleteAiModel(input: DeleteAiModelInput) -> AiModelAssetList`
- `selfTestAiModel(input: SelfTestAiModelInput) -> Notification`
- `getAiProviderStatus() -> AiProviderStatus`
- `saveAiProviderSecret(input: SaveAiProviderSecretInput) -> SaveAiProviderSecretResult`
- `deleteAiProviderSecret(input: DeleteAiProviderSecretInput) -> void`
- `listProcessingJobs(input?: ListProcessingJobsInput) -> ProcessingJobList`
- `pauseProcessingJob(input: ProcessingJobActionInput) -> ProcessingJob`
- `resumeProcessingJob(input: ProcessingJobActionInput) -> ProcessingJob`
- `cancelProcessingJob(input: ProcessingJobActionInput) -> ProcessingJob`
- `retryProcessingJob(input: ProcessingJobActionInput) -> ProcessingJob`
- `getNotifications() -> NotificationList`
- `dismissNotification(input: DismissNotificationInput) -> NotificationList`
- `executeNotificationAction(input: ExecuteNotificationActionInput) -> NotificationList`
- `getRecordingTranscript(input: RecordingAiResultInput) -> RecordingTranscript`
- `getRecordingActivity(input: RecordingAiResultInput) -> RecordingActivity`
- `getRecordingVisualObservations(input: RecordingAiResultInput) -> RecordingVisualObservations`

Expected shared payload concepts:

- `AiSettings`
- `AiProviderKind`
- `AiTranscriptionProviderSettings`
- `AiVisionProviderSettings`
- `AiModelAsset`
- `AiModelDownloadStatus`
- `AiModelSelfTestStatus`
- `ProcessingJob`
- `ProcessingJobStatus`
- `ProcessingChunk`
- `ProcessingChunkStatus`
- `Notification`
- `NotificationAction`
- `TranscriptSourceRole`
- `TranscriptSegment`
- `TranscriptWord`
- `ActivityInterval`
- `ActivitySourceKind`
- `VisualRegion`
- `OcrRegion`
- `VlmObservation`

Expected settings fields:

- `ai.transcription.provider`: `local-whisper-cpp` or `openai-compatible`
- `ai.transcription.localModelId`
- `ai.transcription.remoteBaseUrl`
- `ai.transcription.remoteApiKeyRef`
- `ai.transcription.remoteModel`
- `ai.transcription.wordTimestampsEnabled`
- `ai.vision.provider`: `openai-compatible` or `disabled`
- `ai.vision.baseUrl`
- `ai.vision.apiKeyRef`
- `ai.vision.model`
- `ai.ocr.enabled`
- `ai.ocr.engine`: `tesseract`
- `ai.processing.autoStartAfterRecording`
- `ai.processing.maxParallelJobs`
- `ai.processing.maxParallelChunksPerJob`
- `ai.processing.periodicFullFrameIntervalMs`
- `ai.processing.frameSimilarityEnabled`
- `ai.activity.keyboard.enabled`
- `ai.activity.keyboard.windowMs`
- `ai.activity.mouse.enabled`
- `ai.activity.mouse.windowMs`
- `ai.activity.frameDeltas.enabled`

Settings storage format:

- `app_settings` is a key/value table with one row per setting key.
- `app_settings.value_json` stores typed JSON values.
- `app_settings.is_sensitive` marks encrypted local secret values.
- Sensitive setting values use envelope JSON:
  `{ "v": 1, "alg": "AES-256-GCM", "nonce": "...", "ciphertext": "..." }`.
- AI settings use the existing broad app settings commands. API keys are saved
  through a separate encrypted secret command that returns a secret reference,
  and only that reference is persisted in the visible AI settings object.
- Replacing a remote API key or switching a provider away from remote deletes
  the superseded encrypted secret only after the settings save or reset
  succeeds.
- On Linux, the AES key is stored or unlocked through Secret Service. Unsupported
  platforms return typed not-implemented secret-storage states in M2 Gate A.

## Native Permission Requirements

- Linux/KDE keyboard and mouse activity monitoring must state which native
  interface is used and what user permission, portal, accessibility, or session
  limitation applies.
- If keyboard/mouse activity cannot be captured safely on a platform, the
  adapter returns unavailable and the UI shows activity as unknown or
  frame-delta-only.
- M2 must not capture key values, typed text, clipboard data, or mouse target
  content.
- Keyboard activity records event timing only.
- Mouse activity records timing and coarse movement/button activity only.
- Tesseract and `whisper.cpp` native dependencies must be surfaced in
  Diagnostics or AI provider status.
- Remote endpoint configuration must be explicit because it may upload private
  captured data.

## Privacy and Deletion Contract

- M2 processes private media immediately after recording stop.
- This is a user-approved M2 exception to the original
  redaction-before-processing target.
- M2 must still store enough provenance to support later redaction invalidation.
- Derived records must be deletable or invalidatable by recording ID and time
  range.
- Redaction after M2 must remove or invalidate transcript segments, transcript
  words, OCR regions, VLM observations, activity intervals, frame similarity
  entries, and processing outputs that overlap the redacted span.
- Remote providers may receive captured audio, frame-derived images, OCR text,
  or prompts only after the user configures the remote provider.
- No remote endpoint is used by default.
- API keys must not appear in logs, diagnostics, notifications, or serialized
  frontend state beyond a secret reference or redacted presence indicator.
- Raw audio, raw frames, OCR text, transcripts, prompts, and model responses are
  private content and must not be logged by default.
- Notification text must summarize status without embedding private transcript,
  OCR, or VLM response content.

## Storage Format

M2 adds explicit storage for provider settings, model assets, jobs, and derived
AI output. Exact column names may follow existing Rust naming conventions, but
the schema must preserve these relationships.

Expected new tables:

- `ai_model_assets`
  - model ID, provider, display label, local path, size, checksum or etag when
    known, download status, self-test status, created/updated timestamps
- `notifications`
  - generic source, topic, status, title, message, JSON action descriptors,
    created/updated timestamps, and dismissed timestamp
- `processing_chunks`
  - job ID, recording ID, source kind, segment ID or time range, status,
    attempt count, progress, error code, created/updated timestamps
- `transcript_segments`
  - recording ID, segment ID, track ID, source role, start/end offset,
    provider, model, text, confidence/logprob where available, job/chunk ID
- `transcript_words`
  - transcript segment ID, start/end offset, word text, probability where
    available
- `frame_activity_intervals`
  - recording ID, source kind, start/end offset, score, regions JSON or
    normalized region rows, job/chunk ID
- `ocr_regions`
  - recording ID, frame timestamp, source frame reference, region box, text,
    confidence, provider, model, job/chunk ID
- `vlm_observations`
  - recording ID, frame timestamp, source frame reference, region box, text,
    confidence if available, provider, model, prompt version, job/chunk ID
- `frame_similarity_entries`
  - recording ID, source frame reference, region box, perceptual hash or
    similarity signature, last processed timestamp, job/chunk ID

Existing table changes:

- `app_settings` is migrated to a unified key/value settings table that includes
  record, storage, AI, and encrypted sensitive setting values.
- `processing_jobs.kind` must support M2 job kinds such as transcription,
  activity, OCR, VLM, and composite AI processing if the existing enum remains.
- Existing `capture_events` may be reused for keyboard and mouse activity
  events if it remains the right normalized source of truth.

Migration stop boundary:

- Schema edits and embedded migrations happen in the same implementation gate.
- Migration tests must run with `cargo test --manifest-path
  apps/desktop/src-tauri/Cargo.toml`.
- Migration code must not depend on React.

## Processing Contract

- Processing starts after a recording reaches completed state.
- Processing creates durable jobs and chunks before invoking model providers.
- Recording segments marked failed or incomplete are excluded.
- Each audio source is demuxed or addressed independently.
- Desktop audio and microphone transcripts are persisted separately.
- Transcription output must include segment timestamps.
- Word timestamps are requested when enabled and stored when returned.
- If the provider cannot return word timestamps, transcript segments still
  persist and the UI falls back to segment highlighting.
- OCR and VLM visual processing use the recording's video timeline.
- Frame delta detection plans candidate regions from visual changes.
- Similarity matching avoids reprocessing unchanged regions.
- Similarity matching must not suppress subtle text updates solely because a
  whole-frame similarity score is high.
- The planner must combine multiple signals: changed pixels/regions,
  similarity signatures, OCR-sensitive crops, periodic full-frame checkpoints,
  and abrupt before/after transitions.
- Visual regions are merged or tiled to bounded input sizes before OCR/VLM.
- Periodic full-frame checkpoints are required so stable context is not lost.
- VLM observations must be written as source-linked claims, not free-floating
  summaries.
- VLM responses must be runtime-validated before persistence. Invalid or
  unsupported response shapes fail the chunk with a privacy-safe error instead
  of persisting model text as structured facts.
- Tesseract raw OCR and VLM observations remain distinguishable in storage.
- Partial failures leave failed chunks that can be retried.
- Job progress is based on chunks completed, running, failed, cancelled, and
  queued.

## Local REST Endpoint Contract

- The endpoint binds to `127.0.0.1` only.
- The endpoint uses an ephemeral port selected at runtime.
- The endpoint requires a per-session bearer token.
- The token is never logged.
- The endpoint is started only when local transcription is enabled or needed.
- The endpoint is stopped when the app exits or local transcription is disabled.
- The app consumes the endpoint through the same provider abstraction used for
  remote OpenAI-compatible transcription.
- The endpoint supports at least OpenAI-compatible transcription requests with
  verbose JSON style response data.
- The endpoint response must include text and segment timestamps.
- Word timestamps are included when `whisper.cpp` returns them and the setting
  requests them.
- Upload size, file path, and temporary-file handling must be bounded and
  private-content safe.

## Gate A: AI Contracts, Settings, and Storage

- [x] Add shared AI settings, provider, model asset, processing job,
  notification, transcript, activity, OCR, and VLM result types.
- [x] Extend Rust settings structs, defaults, validation, and limits with AI
  settings.
- [x] Add embedded SQLite migrations for unified key/value app settings, model
  assets, processing chunks, transcript segments, transcript words, activity
  intervals, OCR regions, VLM observations, and frame similarity entries.
- [x] Extend `processing_jobs` support for M2 AI job kinds.
- [x] Add storage helpers for AI settings, encrypted sensitive settings, model
  assets, chunks, and derived result writes.
- [x] Add `processing_jobs` list/get/status helper APIs needed by notification
  action scaffolds.
- [x] Add tests for fresh migration, existing database migration, AI settings
  validation, secret redaction, and derived output provenance.

Status note:

- Commit `6aa1f34` completed the core Gate A storage foundation. The current
  working tree adds the `processing_jobs` list/get/status helpers needed by
  Gate B notification action scaffolds. Gate E still owns concrete job
  creation, claiming, chunk scheduling, and restart recovery.

Exit Criteria:

- AI settings and AI storage contracts are explicit and tested.
- Existing recordings and M1 playback still load after migration.
- React still has no direct SQLite access.

## Gate B: AI Settings UI and Notification Center

- [x] Add an `AI/LLM` tab to Settings using existing shadcn tabs, fields,
  inputs, selects, alerts, and buttons.
- [x] Use shadcn `Select` for Settings and other app select controls instead
  of the temporary native select wrapper.
- [x] Add provider controls for local `whisper.cpp`, remote
  OpenAI-compatible transcription, OCR, VLM, activity tracking, and processing
  concurrency.
- [x] Add visible warnings for remote providers, encrypted local secret storage,
  and unsupported macOS/Windows secret storage.
- [x] Add model cache controls for download, delete, and smoke test.
- [x] Add keyboard and mouse activity toggles with default windows of 30
  seconds and 5 seconds.
- [x] Add the header notification icon and popover with scrollable persistent
  notifications.
- [x] Wire notification actions for pause, cancel, retry, dismiss, and
  model-download retry.
- [x] Add Browser Mode tests for local settings, remote settings, dirty/save
  state, remote warnings, notification popover behavior, and failed/succeeded
  job states.
- [x] Intercept dirty Settings navigation with a save/discard/stay dialog and
  native close/reload protection.

Status note:

- Gate B uses the existing app settings command path for AI settings. It adds
  separate encrypted secret commands for remote provider keys, generic
  notifications, code-defined notification action descriptors, and backend
  allowlist dispatch for notification actions.
- Model download and self-test buttons are active in Gate B, but they create
  unavailable/failed notifications until Gate C implements real model download,
  verification, and `whisper.cpp` smoke testing.

Exit Criteria:

- Users can configure all M2 AI behavior without editing files or environment
  variables.
- Long-running AI work is visible outside Settings.
- The notification center is not a main navigation item.

## Gate C: Model Cache and `whisper.cpp` Local Backend

- [x] Add managed model cache directories under app data.
- [x] Add known local transcription model metadata for multilingual `tiny`,
  `base`, `small`, `medium`, `large-v3-turbo`, and `large-v3`.
- [x] Download only from the pinned official Hugging Face
  `ggerganov/whisper.cpp` GGML files; do not add manual import, arbitrary
  model URLs, or user-facing mirror settings in Gate C.
- [x] Store model files at `app_data/models/whisper.cpp/ggml-<id>.bin`, with
  resumable `.part` files for active or interrupted downloads.
- [x] Persist model download progress, source URL, ETag, total bytes,
  downloaded bytes, error code, checksum algorithm, checksum value, and
  verification status.
- [x] Replace the misleading `sha256` model metadata with explicit checksum
  algorithm plus checksum value because the official model card publishes
  SHA-1 values.
- [x] Implement in-app background model downloads with one active download at a
  time, queued additional downloads, cancellation, retry, resumable partial
  files, and final SHA-1 verification.
- [x] Surface model download progress in both the AI/LLM Settings model rows
  and persistent notifications.
- [x] Add a `model-download-cancel` notification action and keep retry actions
  resumable when the `.part` file still matches source metadata.
- [x] Emit a Tauri `ai-models-changed` event with the full model list after
  every model status or progress change; Settings also refetches on open/focus
  to recover missed events.
- [x] Exclude model files from recording storage retention and size cleanup.
- [x] Make Delete remove the local model file and reset the model row to
  missing while preserving catalog metadata.
- [x] Integrate `whisper.cpp` through the C API as the primary path by
  vendoring release `v1.8.4`, building it through Cargo-driven CMake, and
  generating bindings with build-time bindgen.
- [x] Document CMake and libclang as hard developer/CI build prerequisites.
- [x] Implement load-only model self-test: load the selected model through the
  C API immediately after a successful verified download, immediately unload
  it, and mark the model passed or failed without transcribing audio. Settings
  does not expose a dedicated user-facing smoke-test button in Gate C.
- [x] Add configurable local transcription backend settings for CPU, Vulkan,
  and CUDA. The tracked Cargo default is CPU-only; unavailable GPU backends are
  visible but disabled and cannot be saved. Local developer checkouts may enable
  GPU features without changing package defaults by adding
  `apps/desktop/tauri-dev.local` with `whisper-vulkan` and, when the CUDA
  toolkit is installed, `whisper-cuda`.
- [x] Add capability detection for CPU, Vulkan, and CUDA. Runtime unavailable
  states cover missing model files, failed model loads, and unavailable GPU
  backends; missing CMake/libclang is a build failure, not a runtime state.
- [x] Keep `whisper.cpp` GPU linkage explicit: Vulkan builds link
  `libggml-vulkan.a` and `libvulkan` only when `whisper-vulkan` is enabled;
  CUDA remains a separate opt-in feature because it requires the NVIDIA CUDA
  toolkit at build time.
- [x] Expand AI provider status into structured model/backend status for
  selected backend, available backends, unavailable reasons, selected model
  status, checksum status, and self-test state.
- [ ] Add tests for model cache state transitions, failed downloads, smoke-test
  failure, provider status, Tauri model events, and privacy-safe errors.

Exit Criteria:

- Local multilingual `tiny` can be downloaded, loaded, and smoke-tested on CPU
  as the practical Gate C live validation path. Multilingual `small` remains
  the default selected model and must remain fully cataloged for download and
  load-only self-test.
- GPU controls may be visible only as detected/configurable capability; CPU is
  the required validation path.
- Local Vulkan validation is allowed for developer convenience when the host has
  Vulkan headers, loader, shader tools, and a working GPU driver. CUDA
  validation requires the CUDA toolkit in addition to the NVIDIA runtime.
- Automated tests must not hit Hugging Face or load real Whisper models by
  default. Network and C API behavior are mocked in automated tests.
- If C API integration blocks implementation, the blocker is recorded in the
  scratchpad before any switch to a binary wrapper.
- If a model source lacks checksum or etag metadata, the implementation must
  record the weaker verification behavior in provider status and the scratchpad
  before treating the download path as complete.

## Gate D: Local and Remote Transcription REST Providers

- [x] Implement the loopback-only, token-protected local transcription REST
  endpoint.
- [x] Map local `whisper.cpp` transcription into an OpenAI-compatible verbose
  JSON style response with segment timestamps and optional word timestamps.
- [x] Add the remote OpenAI-compatible transcription provider.
- [x] Add provider adapter logic so internal processing uses REST for both
  local and remote transcription.
- [x] Add request cancellation and timeout handling.
- [x] Add redacted error handling for invalid endpoint, invalid key,
  unavailable local model, oversized input, and provider failure.
- [x] Add tests for loopback binding, token enforcement, no-token rejection,
  provider switching, segment timestamp parsing, word timestamp parsing, and
  private-content-safe logging.

Status note:

- Gate D adds the provider boundary only. Gate E still owns job enqueueing,
  per-track demux/addressing, recording-relative timestamp conversion, durable
  transcript persistence, and job-driven cancellation wiring.

Exit Criteria:

- Processing code can transcribe through local and remote providers without
  changing storage semantics.
- The local endpoint is not reachable without the per-session token.
- Remote provider upload is impossible unless configured.

## Gate E: Processing Job Scheduler and Audio Transcription

- [ ] Implement automatic AI job enqueueing when a recording stops.
- [x] Add manual Library detail start action and Tauri command for recording
  AI processing.
- [x] Implement durable chunk planning for recording segments and separate
  audio tracks.
- [x] Implement app-start recovery that resumes queued or running chunks safely.
- [x] Implement max parallel job limiting and conservative chunk execution that
  does not exceed the chunk limit.
- [x] Implement pause, cancel, retry, and resume behavior.
- [x] Demux or address desktop audio and microphone streams separately.
- [x] Persist transcript segments and transcript words with recording-relative
  timestamps, source role, track ID, provider, model, job ID, and chunk ID.
- [ ] Add tests for one audio track, desktop plus microphone tracks, no audio,
  failed segment exclusion, restart recovery, pause/cancel/retry, and timestamp
  conversion.

Status note:

- Gate E now has manual processing start, durable per-segment/per-audio-source
  chunks, startup recovery, provider readiness checks, notification updates,
  FFmpeg audio extraction, transcription persistence, and a transcript query
  command.
- Automatic post-stop enqueueing remains deferred by product decision. The
  current path refuses to enqueue redacted recordings until redaction-aware
  chunk splitting exists, so cut spans are not processed accidentally.

Exit Criteria:

- A completed recording automatically produces per-track transcript records.
- Restarting the app during processing does not duplicate completed chunks or
  lose queued work.
- Desktop and microphone transcripts remain distinguishable.

## Gate F: Activity Adapters and Activity Lane Data

- [x] Add cross-platform keyboard/mouse activity adapter interfaces.
- [x] Implement Linux/KDE validation path for keyboard and mouse activity
  timing where native permissions and session constraints allow it.
- [x] Add unsupported adapters for platforms that are not implemented in M2.
- [x] Record keyboard and mouse activity timing without key values, typed text,
  clipboard data, or target content.
- [x] Implement frame-delta activity scoring from sampled video frames.
- [x] Merge keyboard, mouse, and frame-delta signals into activity intervals
  using configured activity windows.
- [x] Persist active, idle, and unknown activity intervals with source
  provenance.
- [x] Add tests for default windows, settings changes, frame-only activity,
  input-only activity, unavailable input adapter, and privacy-safe event data.

Exit Criteria:

- Processing produces activity intervals even when native input activity is
  unavailable, using frame deltas as a fallback source.
- Keyboard and mouse activity never stores typed content.
- Library can query activity intervals for a recording.

## Gate G: Visual Delta, OCR, and Guarded VLM Processing

- [ ] Implement frame sampling and source frame references for visual
  processing.
- [ ] Implement conservative delta-region planning with changed regions,
  similarity entries, periodic full-frame checkpoints, and abrupt before/after
  context.
- [ ] Add protections against suppressing subtle text changes due only to high
  whole-frame similarity.
- [ ] Integrate Tesseract for required local structured OCR on selected
  regions.
- [ ] Add OpenAI-compatible VLM provider settings, adapter, guarded request
  path, disabled/unavailable states, timeout handling, and privacy-safe errors.
- [ ] Runtime-validate VLM responses before persistence so only source-linked
  observations with valid timestamps, region provenance, provider/model
  metadata, source frame references, job IDs, and chunk IDs are stored.
- [ ] Persist OCR regions with timestamps, region boxes, provider/model
  metadata, source frame references, job IDs, and chunk IDs.
- [ ] Add retry-safe partial failure handling for OCR and VLM chunks.
- [ ] Add tests for unchanged frames, subtle text changes, abrupt transitions,
  periodic full-frame checkpoints, Tesseract failure, VLM disabled state, VLM
  invalid response shape, remote timeout, and privacy-safe prompts/errors.

Exit Criteria:

- OCR produces source-linked records for a completed recording.
- VLM produces source-linked records when a configured provider is available, or
  clear disabled/unavailable/error states when it is not.
- Unchanged regions are not reprocessed unnecessarily.
- Stable context is still periodically refreshed.

## Gate H: Library Playback AI Integration

- [ ] Extend Library recording detail queries with transcript, activity, OCR,
  and VLM result fetches.
- [ ] Add matching source colors for audio lanes and transcript overlays.
- [ ] Add transcript overlay over the video player with desktop/microphone
  source coloring.
- [ ] Add word-level highlighting when transcript word timestamps exist.
- [ ] Add fallback segment-level highlighting when word timestamps are missing.
- [ ] Add a rolling transcript view in the right-side studio panel.
- [ ] Add clickable transcript segments and visual observations that seek
  playback to source timestamps.
- [ ] Add activity lane/gauge to the timeline.
- [ ] Add OCR display and VLM disabled, unavailable, empty, error, or populated
  observation states with timestamp and region provenance.
- [ ] Add Browser Mode tests for transcript overlay, right-side transcript,
  seeking, word fallback, activity lane, observation clicks, loading, empty,
  error, and narrow viewport behavior.

Exit Criteria:

- A user can watch a recording and see synchronized transcript, activity, OCR,
  and VLM evidence or explicit VLM disabled/unavailable state.
- The transcript source color matches the corresponding audio lane.
- Missing AI output shows clear empty or processing states.

## Gate I: Privacy, Deletion Readiness, and Diagnostics

- [ ] Add provider and dependency diagnostics for local transcription, model
  cache, Tesseract, VLM endpoint, activity adapters, and processing scheduler.
- [ ] Add privacy-safe structured logging for job lifecycle without private
  content.
- [ ] Add derived-data invalidation helpers by recording ID and time range for
  transcript, words, OCR, VLM, activity, frame similarity, and chunk output.
- [ ] Add tests that redaction-style invalidation removes or marks derived
  records for a time range.
- [ ] Add tests or log assertions that transcripts, OCR text, prompts, model
  responses, API keys, and raw media are not logged by default.
- [ ] Update documentation for AI provider behavior, local vs remote data flow,
  model cache location, Linux-only secret encryption caveat, and activity
  monitoring privacy.

Exit Criteria:

- Later scissors/redaction work has concrete invalidation helpers to call.
- Diagnostics expose capability and status without private content.
- Remote upload behavior is documented and visible.

## Gate J: Final Validation and M2 Closeout

- [ ] Run `COREPACK_HOME=/tmp/corepack corepack yarn lint:fix`.
- [ ] Run `COREPACK_HOME=/tmp/corepack corepack yarn lint`.
- [ ] Run `COREPACK_HOME=/tmp/corepack corepack yarn typecheck`.
- [ ] Run `COREPACK_HOME=/tmp/corepack corepack yarn test`.
- [ ] Run `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- [ ] Manually validate local model download and smoke test.
- [ ] Manually validate completed recording auto-processing after stop.
- [ ] Manually validate app restart during processing and successful resume.
- [ ] Manually validate transcript overlay, source colors, word highlighting,
  activity lane, local OCR, and visual observation states in Library playback.
- [ ] Manually validate remote provider warnings and disabled-by-default remote
  behavior.
- [ ] Manually validate a live remote VLM request only when credentials and a
  configured endpoint are available; otherwise record the blocked validation in
  the scratchpad.
- [ ] Record command results, manual validation notes, blocked validations,
  downstream updates, and residual risks in the scratchpad.

Exit Criteria:

- M2 behavior is proven by automated tests and manual validation notes.
- The milestone doc and scratchpad reflect any implementation-driven contract
  changes.
- No gate claims completion without evidence.

## Testing and Validation

- TypeScript unit tests must cover settings formatting, settings store
  behavior, provider client payloads, timeline math, transcript source coloring,
  and notification state transitions.
- Browser Mode tests must cover AI settings, notification popover, job actions,
  transcript playback, activity lane rendering, OCR regions, VLM disabled,
  unavailable, empty, error, or populated states, loading states, and narrow
  viewport layout.
- Rust tests must cover migrations, storage helpers, model cache state,
  loopback REST auth, provider adapters, job scheduler state transitions,
  restart recovery, chunk planning, timestamp alignment, activity windows,
  frame delta planning, OCR/VLM failure handling, and derived-data
  invalidation.
- Privacy tests must assert that private content is absent from default logs and
  notification payloads.
- Manual validation must include local transcription on CPU and local Tesseract
  OCR. GPU/offload and live hosted VLM calls are optional capability
  validations.

## Dependencies and Risks

- `whisper.cpp` C API integration may be harder to build and package than a
  binary wrapper.
- GPU acceleration remains build-environment-sensitive. Unsandboxed local
  validation on 2026-06-09 showed an RTX 4090, working `nvidia-smi`, visible
  `/dev/nvidia*` nodes, and Vulkan detecting the device; earlier failures were
  caused by sandbox device isolation. Vulkan developer prerequisites on
  Arch/CachyOS are `vulkan-headers`, `vulkan-icd-loader`, `vulkan-tools`,
  `shaderc`, `glslang`, and `spirv-tools`. CUDA builds additionally require
  `cuda` plus the NVIDIA runtime packages.
- Tesseract packaging may require native runtime dependencies that differ by
  Linux distribution and by future Windows/macOS support.
- Remote provider support creates real privacy risk. UI copy and defaults must
  make upload behavior unambiguous.
- Hosted remote provider success is not required for M2 closeout unless
  credentials and endpoint configuration are available; the guarded disabled and
  error paths are required.
- Word timestamps are provider-dependent and may not be accurate enough for
  frame-perfect highlighting.
- Visual similarity can miss subtle text changes if implemented with only
  coarse whole-frame similarity.
- Automatic processing before redaction is a deliberate privacy tradeoff and
  must not be hidden.
- Full cross-platform keyboard/mouse activity monitoring is out of scope for
  implementation in M2 despite cross-platform interfaces.

## Assumptions

- M1 recorder output exists as local MKV segments with separate stream indexes
  for screen, desktop audio, and microphone audio when those sources are
  recorded.
- The repository remains a Yarn 4 monorepo using Corepack.
- Rust/Tauri remains the owner of SQLite, native dependencies, files, and
  processing jobs.
- The current first real validation platform remains Linux/KDE Wayland.
- Remote OpenAI-compatible endpoints are user-configured and may be local or
  hosted.
- Local `whisper.cpp` transcription on CPU and local Tesseract OCR are the M2
  provider completion floor.
- VLM output may be disabled or unavailable at closeout when no configured
  provider is available, but the adapter, validation, UI states, and privacy
  guards must exist.
- The default model language behavior should be multilingual because Daydream
  users may not work only in English.
- M2 does not need to prove final transcription accuracy, only useful local
  transcription, timestamped output, and source-linked playback behavior.

## Expected Deliverables

- `AI/LLM` settings tab.
- Header notification popover.
- Managed local transcription model cache.
- Local `whisper.cpp` transcription backend and loopback REST endpoint.
- Remote OpenAI-compatible transcription provider.
- Guarded OpenAI-compatible VLM provider path.
- Tesseract OCR integration.
- Resumable processing job system.
- Per-track transcript storage and playback UI.
- Activity storage and playback lane.
- OCR storage and playback UI plus VLM observation storage and playback states.
- Privacy-safe diagnostics, logs, and invalidation helpers.
