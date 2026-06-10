# Milestone M2: AI Processing Foundation Scratchpad

## Current Status

- Gate A storage foundation landed in commit `6aa1f34`.
- Gate B implementation is complete in the current working tree: AI/LLM
  settings UI, generic notification center, model/provider command scaffolds,
  encrypted remote key command flow, notification action dispatch, dirty
  Settings leave protection, and shadcn Select controls are implemented and
  validated.
- M2 is not closed. Gates A through D have implementation coverage in the
  current milestone history, but the full milestone still requires scheduler,
  activity detection, OCR/VLM processing, playback integration, privacy
  diagnostics, and final validation.
- Gate A is complete enough for Gate B. Gate B added the explicit
  `processing_jobs` list/get/status helper APIs needed by notification action
  scaffolds; Gate E still needs concrete job creation and scheduler ownership.
- Gates C through J are not complete.

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
- Settings use a unified key/value table instead of specialized flat settings
  columns.
- Sensitive settings such as remote API keys are stored as AES-256-GCM
  encrypted base64 envelopes in SQLite, using a Linux Secret Service-backed
  Daydream app key.
- macOS and Windows encrypted secret storage return typed not-implemented
  states until later cross-platform work.
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
- During Gate A planning, user chose:
  - unified key/value settings storage
  - typed JSON rows with `is_sensitive`
  - Linux Secret Service-backed AES-256-GCM envelopes for sensitive settings
  - contracts/storage only, with no AI command stubs in Gate A
  - update milestone docs with the new secret-storage direction
- During Gate B planning, user chose:
  - implement real scaffolds, not doc-only placeholders
  - keep AI settings on the existing broad `getAppSettings`,
    `saveAppSettings`, and `resetAppSettings` path instead of adding narrow
    AI-only settings commands
  - use one dense `AI/LLM` settings tab
  - create a fully generic notification table, shared contract, header center,
    and command surface instead of AI-specific notification aliases
  - store notification actions as code-defined descriptors and dispatch only
    allowlisted backend handlers from `executeNotificationAction`
  - route remote provider API keys through separate encrypted secret commands;
    settings saves persist only returned secret references
  - delete superseded encrypted API keys after the settings save or reset
    succeeds, including provider switches away from remote
  - keep model download and smoke-test buttons active before Gate C by creating
    unavailable/failed notifications rather than disabling the controls
  - replace the temporary native select wrapper with shadcn `Select` across the
    app
  - intercept dirty Settings navigation with Save, Discard changes, and Stay;
    if Save needs storage cleanup confirmation, navigation remains blocked
    until cleanup is confirmed and save succeeds
- During Gate C planning, user chose:
  - download local Whisper GGML models from pinned official Hugging Face
    `ggerganov/whisper.cpp` files only
  - expose only full multilingual `tiny`, `base`, `small`, `medium`,
    `large-v3-turbo`, and `large-v3` model choices; do not expose quantized or
    English-only variants in Gate C
  - keep multilingual `small` as the default selected local transcription model
  - implement real in-app background downloads instead of blocking commands or
    doc-only scaffolds
  - allow one active model download at a time and queue additional requested
    downloads
  - keep resumable `.part` files and use HTTP range plus ETag/length checks
    when retrying interrupted downloads
  - add explicit model-download cancellation from notifications and Settings
  - show model download progress in both AI/LLM Settings rows and persistent
    notifications
  - emit a Tauri `ai-models-changed` event carrying the full model list after
    every model state or progress change, and also refetch on Settings
    open/focus to recover missed events
  - exclude downloaded model files from Daydream recording retention and storage
    size cleanup
  - delete model files immediately when the user deletes a model, then reset
    the model row to missing while preserving catalog metadata
  - use explicit checksum algorithm plus checksum value fields instead of the
    existing misleading `sha256` field, because the official Hugging Face model
    card publishes SHA-1 values
  - implement download verification with pinned SHA-1 values from the official
    model card
  - show safe-specific failure text: URL host, HTTP status where available, and
    validation/error codes, but not raw temporary paths
  - do not add manual model import, arbitrary model URLs, developer URL
    overrides, or user-facing mirror settings in Gate C
  - vendor `whisper.cpp` release `v1.8.4`
  - build the vendored C/C++ code through Cargo-driven CMake
  - use build-time bindgen for C API bindings
  - treat missing CMake or libclang as hard developer/CI build failures and
    document those prerequisites in both the milestone artifacts and README
  - use load-only model self-test in Gate C: load through the C API, immediately
    unload, and do not transcribe audio until Gate D
  - run the load-only self-test automatically after a successful verified model
    download instead of exposing a dedicated Settings smoke-test button
  - update the milestone body so Gate C self-test no longer promises a
    transcription result
  - use multilingual `tiny` for the practical Gate C live validation download
    and load-only self-test; `small` remains the default selected model
  - unload models immediately after self-test instead of keeping them warm
  - add user-facing local transcription backend choices for CPU, Vulkan, and
    CUDA; unavailable GPU options are visible but disabled and cannot be saved
  - keep the tracked Cargo/package defaults CPU-only so CI and fresh developer
    builds do not require GPU SDKs
  - allow developer checkouts to opt into GPU builds locally through ignored
    `apps/desktop/tauri-dev.local` or `DAYDREAM_TAURI_FEATURES`; this checkout
    enables `whisper-vulkan` locally and leaves `whisper-cuda` commented until
    the CUDA toolkit is installed
  - default the saved local transcription backend setting to CPU
  - for Linux Gate C, consider CPU, Vulkan, and CUDA the supported backend
    choices for `whisper.cpp`; other upstream platform-specific backends remain
    out of scope for this gate
  - link `libggml-vulkan.a` and `libvulkan` only when the `whisper-vulkan`
    feature is enabled; CPU builds must not pull Vulkan link requirements
  - reject saving unavailable GPU backends and coerce any existing invalid saved
    backend values back to CPU on load
  - expand AI provider status into structured selected backend, available
    backend, unavailable reason, selected model, checksum, and self-test state
    data
  - use `reqwest` async streaming for model downloads
  - persist download progress and error detail by extending `ai_model_assets`
    rather than adding a separate download-attempt table
  - do not run automated tests against Hugging Face or real Whisper model files;
    tests should mock HTTP and C API behavior
- During Gate D planning and implementation, user chose:
  - implement a Rust-owned loopback transcription endpoint instead of exposing
    a public local API
  - keep the endpoint bound to `127.0.0.1` on an ephemeral port with a
    per-session bearer token
  - use the OpenAI-compatible `/v1/audio/transcriptions` multipart request
    shape for both local and remote transcription
  - request `verbose_json` plus segment timestamps for every provider request,
    and word timestamps only when the setting requests them
  - map the current selected transcription model ID through the provider
    adapter; Gate E may add a dedicated remote transcription model setting if
    real provider validation shows that local and remote model IDs need to
    diverge
  - keep remote transcription disabled unless the user selected the remote
    provider and configured both endpoint and encrypted API key reference
  - handle cancellation in the REST adapter with a shared cancellation flag;
    Gate E still owns job-state propagation into that flag
  - keep provider errors privacy-safe by not surfacing response bodies, API
    keys, raw audio bytes, transcripts, or model output in default errors

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
- Existing AI model command scaffolds already provide `listAiModels`,
  `downloadAiModel`, `deleteAiModel`, `selfTestAiModel`, and
  `getAiProviderStatus`, but Gate C must replace the unavailable/failed
  placeholders with real model cache behavior.
- Existing frontend notification event precedent uses kebab-case event names
  such as `recorder-status-changed`; Gate C should follow that pattern with
  `ai-models-changed`.
- Gate D added `reqwest` multipart support and a direct `tokio` dependency for
  cancellable REST waits. No external HTTP server crate was added.

## External Evidence

- `whisper.cpp` provides local Whisper inference, model download tooling,
  quantization, CPU and GPU-oriented backends, C API surface, and server
  examples.
- The official Hugging Face `ggerganov/whisper.cpp` model card lists the full
  multilingual models needed by Gate C and publishes SHA-1 values:
  - `tiny`: 75 MiB,
    `bd577a113a864445d4c299885e0cb97d4ba92b5f`
  - `base`: 142 MiB,
    `465707469ff3a37a2b9b8d8f89f2f99de7299dac`
  - `small`: 466 MiB,
    `55356645c2b361a969dfd0ef2c5a50d530afd8d5`
  - `medium`: 1.5 GiB,
    `fd9727b6e1217c2f614f9b698455c4ffd82463b4`
  - `large-v3-turbo`: 1.5 GiB,
    `4af2b29d7ec73d781377bfd1758ca957a807e941`
  - `large-v3`: 2.9 GiB,
    `ad82bf6a9043ceed055076d0fd39f5f186ff8062`
- GitHub release metadata identified `whisper.cpp` `v1.8.4` as the pinned Gate C
  release target on June 9, 2026.
- `whisper.cpp` supports more backends than Gate C exposes. The Gate C Linux
  settings surface is intentionally limited to CPU, Vulkan, and CUDA.
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
- Build-time bindgen makes libclang a hard development and CI prerequisite for
  Gate C.
- Cargo-driven CMake makes CMake a hard development and CI prerequisite for
  Gate C.
- GPU/offload support remains build-environment-sensitive. Unsandboxed
  validation on 2026-06-09 showed the host has a working RTX 4090 GPU stack:
  `nvidia-smi` succeeded, `/dev/nvidia*` nodes were visible, loaded NVIDIA
  modules were present, and `vulkaninfo --summary` detected `NVIDIA GeForce RTX
  4090`. Earlier failures were caused by sandbox device isolation.
- Vulkan may be available on systems where CUDA is unavailable; Settings must
  disable unavailable GPU backends rather than saving them.
- Vulkan developer packages on Arch/CachyOS: `vulkan-headers`,
  `vulkan-icd-loader`, `vulkan-tools`, `shaderc`, `glslang`, and
  `spirv-tools`. CUDA builds additionally require `cuda` plus the NVIDIA runtime
  packages.
- Large model downloads can consume significant bandwidth and disk space. Gate C
  does not ask for confirmation before download, so Settings rows must show
  size and source clearly before the Download action.
- Tesseract packaging and runtime discovery may be platform-specific.
- Remote provider API compatibility may vary across OpenAI-compatible vendors.
- Chunk-level retry semantics need careful idempotency to avoid duplicate
  transcript/OCR/VLM records.
- Restart recovery needs durable scheduler state and clear stale-running job
  handling.
- Loopback REST endpoint lifecycle must be tied to app lifecycle to avoid
  dangling local services.
- The Gate D local endpoint currently serializes accepted requests on one
  listener thread. That is acceptable until Gate E introduces chunk scheduling;
  Gate E must decide whether local transcription concurrency is controlled by
  scheduler parallelism, endpoint workers, or both.

## Stop Boundaries

- If `whisper.cpp` C API binding fails due to build or packaging portability,
  stop and document the blocker before switching to a vendored binary wrapper.
- If a remote provider stores or transmits private data in a way not visible to
  the user, stop and update the milestone contract before implementing it.
- If Linux Secret Service cannot provide or persist the Daydream app encryption
  key, sensitive settings must fail with a typed unavailable error rather than
  falling back to plaintext or obscured local storage.
- If VLM output cannot be runtime-validated into source-linked observations,
  stop before persisting it as structured fact.
- If model download sources lack checksum or etag metadata, record the weaker
  verification behavior in provider status and the scratchpad before treating
  the download path as complete.
- If the official model source changes a pinned checksum, fail verification and
  show a safe-specific error instead of accepting the file.
- If a requested GPU backend is unavailable at save time, reject the save or
  coerce stale stored values to CPU rather than preserving an unusable backend.
- If build-time bindgen or CMake integration blocks normal Rust builds, stop and
  record the blocker before switching to checked-in bindings, feature gating, or
  a binary wrapper.
- If keyboard/mouse activity capture requires privileged or hidden monitoring,
  stop and record the native permission requirement before enabling it.
- If schema work changes redaction invalidation semantics, update the milestone
  body before completing that gate.
- If visual processing needs persistent cropped images for practical reasons,
  update the milestone body before storing them.

## Validation Log

- Commit `6aa1f34` added tested shared AI contracts, unified key/value settings
  storage, encrypted sensitive setting envelopes, M2 storage schema, M2
  processing job kind support, model/chunk/derived-output storage helpers, and
  settings/UI test coverage updates.
- Validation commands from `6aa1f34` are not recorded in this scratchpad. Re-run
  the relevant TypeScript and Rust gates before treating Gate A as closed.
- Current Gate B validation:
  - `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml -- --check`
    passed.
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` passed:
    181 Rust tests.
  - `COREPACK_HOME=/tmp/corepack corepack yarn typecheck` passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn lint` passed with only existing
    max-lines warnings in `recorder-workspace-store.ts`, `DiagnosticsScreen.tsx`,
    and `LibraryScreen.tsx`.
  - `COREPACK_HOME=/tmp/corepack corepack yarn workspace @daydream/desktop test
    src/features/ai/ai-client.test.ts
    src/features/notifications/notifications-client.test.ts
    src/features/notifications/notifications-store.test.ts
    src/features/notifications/components/notification-center.browser.test.tsx
    src/screens/SettingsScreen.browser.test.tsx
    src/screens/LibraryScreen.browser.test.tsx
    src/features/recorder/scene-layout.test.ts
    src/features/recorder/recorder-workspace-store.test.ts
    src/features/settings/settings-store.test.ts` passed: 9 files, 113 tests.
  - `COREPACK_HOME=/tmp/corepack corepack yarn workspace @daydream/shared test`
    passed: 1 file, 9 tests.
  - `git diff --check` passed.
  - Root `COREPACK_HOME=/tmp/corepack corepack yarn test` is not a reliable
    green command for filtered Gate B tests because the same file filter is
    forwarded to `@daydream/ffmpeg`, whose `vitest run` command exits 1 when no
    matching test files exist.
- Latest review after Gate B found no discrete correctness issues. TypeScript
  typecheck, ESLint, and Rust tests passed; the JS test run hit a sandbox
  listen `EPERM` after tests passed.
- Current Gate D validation:
  - `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - Elevated `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml`
    passed: 216 Rust tests. Elevation was required because the sandbox denied
    loopback socket binding with `EPERM`.
  - `COREPACK_HOME=/tmp/corepack corepack yarn lint` passed with only existing
    max-lines warnings in `recorder-workspace-store.ts`, `DiagnosticsScreen.tsx`,
    and `LibraryScreen.tsx`.
  - `COREPACK_HOME=/tmp/corepack corepack yarn typecheck` passed.
  - Sandboxed `COREPACK_HOME=/tmp/corepack corepack yarn test` failed before
    app assertions because Vitest could not bind `::1` with `EPERM`.
  - Elevated `COREPACK_HOME=/tmp/corepack corepack yarn test` ran 25 files and
    240 tests, but failed two existing `CaptureScreen.browser.test.tsx`
    assertions around canvas preset and audio picker behavior. These failures
    are outside Gate D's Rust transcription provider path.

## Gate D Closeout Notes

- Completion proof: Rust unit tests cover loopback binding, token-required and
  wrong-token rejection, valid local verbose JSON response mapping, remote
  multipart upload shape, bearer auth, safe auth failure errors, cancelled REST
  requests, local disconnect propagation into the Whisper cancel flag, local
  missing-model typed errors, silent empty transcripts, segment timestamp
  validation, word timestamp parsing, binary multipart payload preservation, and
  WAV input validation.
- Privacy proof: provider request failures are mapped to typed errors without
  response bodies, API keys, raw audio content, transcripts, or model response
  text. The local bearer token is generated per server instance and is not
  logged.
- Downstream reconciliation: Gate E remains responsible for durable job
  scheduling, demuxing/addressing separate audio tracks, converting provider
  timestamps to recording-relative offsets, persisting transcript records, and
  propagating job cancellation into the Gate D cancellation flag. No Gate F-J
  deliverables need changes from Gate D.

## Deferred Follow-Up Candidates

- Full scissors/redaction UI before processing.
- Daily digest generation.
- Task/todo extraction.
- Embeddings and vector search.
- Chat over recordings.
- Speaker diarization.
- macOS and Windows encrypted secret storage for remote API keys.
- Public local API endpoint for external tools.
- Full cross-platform native activity monitoring.
- Accuracy benchmark suite across languages, accents, and noisy audio.
- Installer-level packaging for model files, Tesseract, and native OCR/model
  dependencies.
