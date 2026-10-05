# Refactoring Tasks

Active cleanup work is listed below. Older checked sections record completed
2026-06-12 work and historical validation; they do not define a current goal or prove
today's checks. Add focused ownership changes when touching complex owners.

## Completed cleanup: full-frame OCR documents

- [x] Native OCR storage/contract ownership. Group ordered document blocks/content,
  temporal occurrences, selected-frame/paginated queries and legacy reconstruction
  under a focused storage owner. Add explicit tested migration and preserve existing
  raw-row compatibility. Risks: hidden-span history, unstable cursors/provenance and
  oversized whole-recording payloads. Record final counts and focused/native gates.
  Implemented focused storage owners: barrel 15, content 97, write 100, read 109,
  query 188, timeline 61, legacy 128, types 69 lines; native DTOs 121 and commands 86. Tests split
  by cache/query/persistence/legacy workflows. Schema 20 is additive. Shared lint/
  typechecks pass. Native full unit suite passed 381 tests before finishing changes;
  final OCR checks passed 40 tests, with formatting, doc-only retry and strict
  all-target Clippy also passing. Sets overlap and are not summed.
- [x] Recognition/processing ownership. Separate literal full-frame OCR from
  descriptive VLM context, retain Tesseract hierarchy, pin provider settings and
  isolate exact-image/content identity. Risks: remote opt-in leakage, fabricated
  continuity, mixed-provider retries and unrelated audio reprocessing. Document
  baseline owners at `51ad981`: OCR parser 296 lines, frame jobs 353 and descriptive
  VLM adapter 419. Keep literal OCR requests out of the already broad descriptive
  adapter; record final focused owners from the implementation owner. Verify focused
  provider/cache/revision/redaction/recovery scenarios, then coordinated native gates.
  Implemented OCR domain owners: orchestrator 355, configuration 137, extraction
  218, Tesseract 228, literal Vision 286 and OCR-only reprocess 39 lines; enqueue 389.
  Existing settings persistence 435 is a flat declarative map with no new lifecycle
  owner. OCR filtered checks pass 33; AI 98/settings 11 ownership checks pass with
  overlap. Schema/extraction/cache-resumption reds are recorded; no blanket red-first
  claim. Finishing cache-reuse and permit changes have focused coverage, and final
  formatting, doc-only and strict Clippy checks pass.
- [x] OCR UI/Settings ownership. Compose Text/Changes/Details from the published
  typed contract in focused feature owners, keep Inspector and route thin, and show
  uncertainty/legacy/provider failures honestly. Verify actual both-theme desktop/
  narrow selection/history/retry workflows plus shared settings save states and the
  final JavaScript/build gate. Baseline visual Inspector 156 lines and Vision card
  272; full-document/history/engine controls need their own owners rather than
  growing these mixed surfaces. Keep exact wire definitions in shared/native code.
  Implemented readable OCR composition: visual Inspector 94 lines delegates
  Text/Changes/Details to nine focused OCR feature files; API/decoder/query/revision
  ownership lives in six AI OCR files. Production owners remain under 400 lines.
  Focused browser/unit checks pass, and final JS gates pass 348 tests, all lint/
  typechecks and production build. Independent Inspector is 4/5 in both themes
  after three passes; Settings is 4/5 after one. The timeline uses separate per-sample
  metadata and filled navigation cells, with physical midpoint clicks covered at
  desktop/narrow widths in both themes. All final gates have passed.

Approved behavior: [OCR documents design](ocr-documents-design.md).

## Completed master-review cleanup: 2026-10-05

- [x] Native protected-window/lifecycle ownership. Before: `window_protection.rs`
  397 lines mixes matching, dwell/merge state, persistence and monitoring;
  `lifecycle_commands.rs` 407 lines mixes manual/automatic lifecycle and completion.
  Split matching/state/persistence/monitor/tests and lifecycle supervision; manager
  owns pause authority. Risks: metadata/storage failure leaks, interrupted protection
  across rule changes and stale automatic resume ownership. Verify focused
  `recorder::` scenarios, then coordinated native gate with two build jobs.
  Verified: lifecycle 209; automatic/completion/attachments/supervision 104/79/80/66;
  pause/stop 130/257; protection barrel 9 with matching/state/persistence/monitor
  68/132/116/112 and tests 245; PipeWire session/intake 395/108; KWin provider/script
  458/147. Full 349 native tests, formatting and strict all-target Clippy pass.
  Real hardware and packaged capture remain manual limits in the native report.
- [x] Processing privacy/context/deletion ownership. Before: visual `mod.rs` 737
  lines mixes orchestration, OCR/VLM batches and context. Extract focused context,
  shared current privacy eligibility, redaction identity mapping and deletion artifact
  service. Risks: hidden media processing/upload, raw redacted context, stranded
  restart chunks and temp leftovers. Verify focused AI/redaction/deletion/recovery
  scenarios, then coordinated native gate.
  Implemented: visual root 313 lines (was 737), frame jobs 353, context 78,
  privacy eligibility 194 and deletion service 128. Focused ownership tests pass
  118 (AI 71, Library 24, retention 3, transcription 17, app-data lease 1,
  deletion 2). Final 349 native tests, formatting and strict Clippy pass; finishing
  AI/transcription checks pass 94 after the final bounded cancellation/Clippy edits.
- [x] Local transcription cancellation boundary. Extract request receipts, bounded
  request admission and private scratch cleanup so cancellation acknowledges real
  handler completion before redaction/deletion proceeds. Final focused owners:
  receipts 136, requests 28, scratch 79, server 364 and worker 432 lines. The worker
  remains the model/process cache owner; its small kill/wait change keeps that
  boundary. The drain regression was observed failing before its fix, and 22
  transcription tests initially passed; the final transcription set passes 23.
  The failed-parent cancellation regression was observed red then green. Shared
  native gates pass, with 94 focused AI/transcription checks covering finishing edits.
- [x] Frontend async draft, route reconciliation and privacy cache ownership.
  Preserve newer draft edits, clear stale Inspector results after redaction and
  reconcile missed recorder events on return. Sort visual frames once instead of
  per rendered frame. Verify focused unit/browser failure and race scenarios, then
  one JavaScript gate. Final ownership/counts follow the frontend fix report.
  Implemented: result visibility/read-hook owners 175/65 lines, route controller 322,
  recorder lifecycle/persistence 193/195, settings store/controller 231/277.
  Focused tests pass 50 unit and 95 browser; typechecks and targeted autofix pass.
  Synthetic 10,000-frame construction fell from 2,366 ms to 12 ms. Final JS gate
  passed: lint/typecheck, desktop 31 files/280 tests, shared 9 and FFmpeg 1
  (290 total), plus production build. Known eager app chunk: 859 kB/256 kB gzip.
  Next untouched owners: protected-window card 614, track lane 495.

Current findings and fix evidence: [master review](reviews/master-review.md).
Exact commands/resource limits: [validation](agent-validation.md).

## Completed cleanup history

## Operating Rules

- [x] Preserve user-facing behavior unless a task explicitly says otherwise.
- [x] Preserve Tauri command names, event names, IPC payload shapes, and SQLite
      schema unless a task explicitly adds a migration.
- [x] Keep privacy-sensitive logging rules intact: no raw frames, audio, OCR
      text, transcripts, window titles, model prompts, embeddings, or summaries
      in logs by default.
- [x] Keep source moves behavior-preserving first, then clean internal APIs
      where it reduces coupling or repeated argument lists.
- [x] Move tests with the logic they protect. Do not drop coverage during
      module splits.
- [x] Run focused verification after each completed task and record the command
      result in this file.

## Initial Offenders

### 1. Split Rust AI ownership

Source: `apps/desktop/src-tauri/src/ai/`

Current problems:

- Root `ai/mod.rs` is down to `203` lines after moving DTOs, command
  entrypoints, model/download ownership, processing planning, processing
  worker orchestration, temp-file cleanup, audio/activity/visual source
  processing, result/read-side mapping, and tests out. It now owns module
  wiring plus shared error/cancel/time utilities.
- Mixes IPC DTOs, Tauri commands, model catalog/download management, recovery,
  processing workers, chunk planning, OCR, VLM calls, transcript persistence,
  notification wiring, read-side mapping, utility helpers, and tests.
- Clippy previously reported avoidable issues in this file; those were cleaned
  up before the ownership split.

Tasks:

- [x] Create `apps/desktop/src-tauri/src/ai/mod.rs` as the public module entry.
- [x] Move serialized DTOs, command inputs, and `AiError` into `ai/types.rs`.
- [x] Move Tauri command functions into `ai/commands.rs`.
- [x] Move model catalog, `AiModelManager`, download/recovery/delete/self-test,
      checksum, and model path logic into `ai/models.rs`.
- [x] Move processing worker orchestration, job actions, chunk planning,
      completion/failure handling, and temp-file cleanup into
      `ai/processing/`.
- [x] Split processing internals by source kind: audio, activity, visual.
- [x] Move transcript/activity/visual read-side mapping into `ai/results.rs`.
- [x] Replace repeated chunk status/error handling with a small internal
      processing context and shared completion helpers.
- [x] Keep Tauri IPC command names unchanged. `lib.rs` handler paths now point
      at `ai::commands::*` because Tauri macro helper symbols are generated in
      the module where each command function is defined.
- [x] Move existing AI tests beside the modules they cover or into
      `ai/tests.rs`.
- [x] Verify with `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- [x] Verify with `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- [x] Verify with `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`.
- [x] Verify with `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml`.

Verification notes:

- Partial progress on 2026-06-12:
  - Removed the three trivial clippy warnings in `ai.rs`.
  - `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings.
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` passed with
    loopback binding permission: 277 tests.
  - The ownership split remains pending.
- Module root/types/test split on 2026-06-12:
  - Moved `ai.rs` to `ai/mod.rs`.
  - Added `ai/types.rs` for public IPC DTOs, command inputs, result structs,
    visual/transcript/activity DTOs, and `AiError`.
  - Moved the existing AI tests into `ai/tests.rs`.
  - Current AI line counts: `mod.rs` 6,464, `types.rs` 347, `tests.rs` 1,891,
    `notification_helpers.rs` 346.
  - `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings.
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml ai::tests`
    passed with local test permission: 44 tests.
  - Model/download, command, processing, and result ownership splits remain
    pending.
- Command split on 2026-06-12:
  - Added `ai/commands.rs` for Tauri command entrypoints and startup worker
    kickoff.
  - Updated `lib.rs` handler paths from `ai::*` to `ai::commands::*`; external
    command names remain unchanged because the existing `#[tauri::command]`
    rename attributes moved with the command functions.
  - Current AI line counts: `mod.rs` 5,924, `commands.rs` 542,
    `types.rs` 347, `tests.rs` 1,891, `notification_helpers.rs` 346.
  - `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings.
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml ai::tests`
    passed with local test permission: 44 tests.
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` passed with
    loopback binding permission: 277 tests.
  - Model/download, processing, and result ownership splits remain pending.
- Model/download split on 2026-06-12:
  - Added `ai/models.rs` for `AiModelManager`, the local Whisper catalog,
    download recovery, resumable downloads, cancellation/finalization,
    model asset persistence, cache paths, checksums, and model-list events.
  - Current AI line counts: `mod.rs` 4,889, `models.rs` 1,045,
    `commands.rs` 542, `types.rs` 347, `tests.rs` 1,891,
    `notification_helpers.rs` 346.
  - `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings.
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml ai::tests`
    passed with local test permission: 44 tests.
  - Processing and result ownership splits remain pending.
- Processing planning split on 2026-06-12:
  - Added `ai/processing/mod.rs` and `ai/processing/planning.rs` for audio,
    activity, and visual chunk planning, playback-offset mapping, and audio
    track creation.
  - Current AI line counts: `mod.rs` 4,662, `models.rs` 1,045,
    `processing/planning.rs` 233, `commands.rs` 542, `types.rs` 347,
    `tests.rs` 1,891, `notification_helpers.rs` 346.
  - `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings.
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml ai::tests`
    passed with local test permission: 44 tests.
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` passed with
    loopback binding permission: 277 tests.
  - Processing worker/chunk execution, audio/activity/visual processing
    internals, and result ownership splits remain pending.
- Processing worker and temp cleanup split on 2026-06-12:
  - Added `ai/processing/worker.rs` for stale-job recovery, worker ticks, job
    enqueueing, chunk claiming/dispatch wrappers, pause/cancel/retry actions,
    transcription readiness checks, and completion/failure notification
    handling.
  - Added `ai/processing/temp_files.rs` for processing temp audio paths, visual
    temp directories, and RAII cleanup guards.
  - Kept `ai::enqueue_recording_ai_processing` crate-visible because
    `recorder::commands` starts AI processing after recording completion.
  - Current AI line counts: `mod.rs` 3,254, `processing/worker.rs` 1,313,
    `processing/temp_files.rs` 102, `processing/planning.rs` 233,
    `models.rs` 1,045, `commands.rs` 542, `types.rs` 347,
    `tests.rs` 1,891, `notification_helpers.rs` 346.
  - `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings.
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml ai::tests`
    passed: 44 tests.
  - Full `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` failed
    under sandboxed loopback restrictions, then passed with loopback permission:
    276 tests.
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` failed under
    the default sandbox only because 9 transcription tests could not bind
    loopback ports; 268 tests passed before that failure. The escalation request
    for the same full command timed out twice in the automatic approval review,
    so the full loopback-enabled pass remains pending for this checkpoint.
  - Audio/activity/visual processing internals and result ownership splits
    remain pending.
- Source-kind processing split on 2026-06-12:
  - Added `ai/processing/audio.rs` for audio chunk execution, FFmpeg audio
    extraction, silence-gated transcription ranges, transcript persistence, and
    audio helper tests.
  - Added `ai/processing/activity.rs` for activity chunk execution, keyboard
    and mouse event interval mapping, frame-delta activity extraction, interval
    merging, and activity helper tests.
  - Added `ai/processing/visual.rs` for visual chunk execution, frame
    extraction, frame similarity/delta regions, Tesseract OCR parsing, VLM
    request/validation, and visual output persistence.
  - Kept test-only re-exports in `ai/processing/mod.rs` for existing
    scenario-first AI tests while keeping normal production exports narrower.
  - Current AI line counts: `mod.rs` 1,010, `processing/worker.rs` 1,338,
    `processing/visual.rs` 1,283, `models.rs` 1,045,
    `processing/audio.rs` 489, `processing/activity.rs` 450,
    `commands.rs` 542, `types.rs` 347, `processing/planning.rs` 233,
    `processing/temp_files.rs` 102, `tests.rs` 1,891,
    `notification_helpers.rs` 346.
  - `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings.
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml ai::tests`
    passed: 44 tests.
  - Full `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml`
    remains pending for this checkpoint because the default sandbox blocks
    loopback ports used by transcription tests and the escalation approval
    timed out twice during the previous full-suite attempt.
  - Result/read-side mapping remains pending.
- Results split on 2026-06-12:
  - Added `ai/results.rs` for transcript, activity, visual observation, VLM
    state, processing-job DTO, processing-state DTO, and redaction-aware
    read-side filtering.
  - Kept `ai::visible_transcript_segments_for_recording` and
    `ai::processing_state_for_recording` crate-visible because `library.rs`
    already uses them.
  - Current AI line counts: `mod.rs` 203, `processing/worker.rs` 1,338,
    `processing/visual.rs` 1,283, `models.rs` 1,045, `results.rs` 818,
    `commands.rs` 542, `processing/audio.rs` 489,
    `processing/activity.rs` 450, `types.rs` 347,
    `notification_helpers.rs` 346, `processing/planning.rs` 233,
    `processing/temp_files.rs` 102, `tests.rs` 1,891.
  - `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings.
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml ai::tests`
    passed: 44 tests.
  - Full `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml`
    remains pending for this checkpoint for the same loopback sandbox reason
    noted above.
  - The remaining AI cleanup is to reduce repeated chunk status/error handling
    with a small processing context/shared completion helper.
- Processing status helper cleanup on 2026-06-12:
  - Added an internal `ProcessingChunkContext` in `ai/processing/worker.rs` for
    chunk status transitions, job reloads, progress notifications, and
    processing-state emission.
  - Replaced repeated audio/activity/visual wrapper status writes for success,
    pause requeue, cancellation, and failure with shared context methods while
    keeping the existing source-specific tracing event names.
  - Current AI line counts: `mod.rs` 203, `processing/worker.rs` 1,243,
    `processing/visual.rs` 1,283, `models.rs` 1,045, `results.rs` 818,
    `commands.rs` 542, `processing/audio.rs` 489,
    `processing/activity.rs` 450, `types.rs` 347,
    `notification_helpers.rs` 346, `processing/planning.rs` 233,
    `processing/temp_files.rs` 102, `tests.rs` 1,891.
  - `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings.
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml ai::tests`
    passed: 44 tests.
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` passed with
    loopback binding permission: 277 tests.
  - AI ownership split is complete for the documented scope. Any further AI
    cleanup should be tracked as a newly discovered task instead of extending
    this finished split indefinitely.

### 2. Split transcription ownership

Source: `apps/desktop/src-tauri/src/transcription/`

Current problems:

- Root `mod.rs` is down to `130` lines after splitting provider, HTTP, REST,
  local server, local worker, protocol, types, and tests.
- The previous top-level `#![allow(dead_code)]` has been removed.
- Mixes provider state, local worker process management, local loopback HTTP
  server, multipart parsing, remote REST calls, validation, and tests.
- Clippy previously reported a simplifiable match and a public function with
  too many arguments; those were cleaned up before the ownership split.

Tasks:

- [x] Create `apps/desktop/src-tauri/src/transcription/mod.rs`.
- [x] Move request/result/error structs into `transcription/types.rs`.
- [x] Move `TranscriptionProviderManager` and provider selection into
      `transcription/provider.rs`.
- [x] Move in-process and process-backed Whisper runner logic into
      `transcription/local_worker.rs`.
- [x] Move loopback transcription server logic into
      `transcription/local_server.rs`.
- [x] Move local HTTP parsing and multipart helpers into
      `transcription/http.rs`.
- [x] Move OpenAI-compatible REST transcription into `transcription/remote.rs`.
- [x] Replace the 9-argument `transcribe_remote(...)` internal API with a
      `RemoteTranscriptionRequest` struct.
- [x] Remove top-level `#![allow(dead_code)]`; add narrow allows only if
      justified.
- [x] Move existing transcription tests beside the modules they cover or into
      `transcription/tests.rs`.
- [x] Verify with `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- [x] Verify with `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- [x] Verify with `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`.
- [x] Verify with `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml`.

Verification notes:

- Partial progress on 2026-06-12:
  - Replaced the 9-argument `transcribe_remote(...)` with
    `RemoteTranscriptionRequest`.
  - Simplified the clippy-flagged single-pattern `match`.
  - `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings after the follow-up `ai.rs` cleanup.
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml remote_`
    passed with loopback binding permission: 4 tests.
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` passed with
    loopback binding permission: 277 tests.
  - The same filtered Rust tests failed under the default sandbox because the
    one-shot test server could not bind a local port.
- Module split progress on 2026-06-12:
  - Moved the module root to `transcription/mod.rs`.
  - Added `transcription/types.rs`, `transcription/http.rs`,
    `transcription/remote.rs`, and `transcription/local_server.rs`.
  - Removed the broad top-level dead-code allow from the transcription module.
  - `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings.
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml transcription::tests`
    passed with loopback binding permission: 16 tests.
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` passed with
    loopback binding permission: 277 tests.
  - `rg "allow\\(dead_code\\)|allow\\(clippy::" apps/desktop/src-tauri/src/transcription`
    found no remaining suppressions.
  - Provider manager, local worker runner, and test relocation are still
    pending.
- Local worker and test split progress on 2026-06-12:
  - Added `transcription/local_worker.rs` for in-process and process-backed
    Whisper runners.
  - Added `transcription/local_protocol.rs` for local loopback/worker JSON
    request and ready-message shapes.
  - Moved transcription tests into `transcription/tests.rs`.
  - Current line counts: `mod.rs` 393, `local_worker.rs` 399,
    `local_server.rs` 346, `http.rs` 310, `remote.rs` 275, `types.rs` 75,
    `local_protocol.rs` 17, `tests.rs` 690.
  - `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings.
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml transcription::tests`
    passed with loopback binding permission: 16 tests.
- Provider manager completion on 2026-06-12:
  - Added `transcription/provider.rs` for `TranscriptionProviderManager`,
    provider selection, local warmup validation, and remote API-key lookup.
  - Final transcription line counts: `mod.rs` 130, `provider.rs` 275,
    `local_worker.rs` 399, `local_server.rs` 346, `http.rs` 310,
    `remote.rs` 275, `types.rs` 75, `local_protocol.rs` 17,
    `tests.rs` 701.
  - `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings.
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml transcription::tests`
    passed with loopback binding permission: 16 tests.
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` passed with
    loopback binding permission: 277 tests.
  - `rg "allow\\(dead_code\\)|allow\\(clippy::" apps/desktop/src-tauri/src/transcription`
    found no remaining suppressions.

### 3. Split Linux capture ownership

Source: `apps/desktop/src-tauri/src/recorder/platform/linux/capture/`

Current state:

- Split files total `2,603` lines.
- `capture/mod.rs` is down to `166` lines and owns module wiring plus shared
  PipeWire format parameter helpers.
- The previous top-level `#![allow(dead_code)]` and
  `#[allow(clippy::too_many_arguments)]` suppressions have been removed.
- Existing tests live in `capture/tests.rs`.

Tasks:

- [x] Create `apps/desktop/src-tauri/src/recorder/platform/linux/capture/mod.rs`.
- [x] Move capture and preview session lifecycle into `capture/session.rs`.
- [x] Move scene-source to live-stream matching into `capture/preflight.rs`.
- [x] Move frame scheduling and shutdown handling into `capture/scheduler.rs`.
- [x] Move latest-frame sampling and log summaries into `capture/sampler.rs`.
- [x] Move PipeWire CPU and DMA-BUF frame extraction into
      `capture/frame_convert.rs`.
- [x] Move preview frame path into `capture/preview.rs`.
- [x] Replace `too_many_arguments` suppressions with `DmaBufFrameInput` and
      `CpuReadableFrameInput` structs.
- [x] Remove top-level `#![allow(dead_code)]`; add narrow allows only if
      justified.
- [x] Move existing Linux capture tests beside the modules they cover or into
      `capture/tests.rs`.
- [x] Verify with `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- [x] Verify with `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- [x] Verify with `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`.
- [x] Verify with `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml`.

Verification notes:

- Preflight split on 2026-06-12:
  - Added `recorder/platform/linux/capture/preflight.rs` for
    `LiveCapturePreflight`, `PreparedLiveCapture`, scene-source to live-stream
    matching, live capture preparation, and preview/live segment start
    orchestration.
  - `capture.rs` remains the parent file module for now; the explicit
    `capture/mod.rs` conversion remains pending.
  - Current Linux capture line counts: `capture.rs` 2,336,
    `capture/preflight.rs` 250.
  - `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings.
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml recorder::platform::linux::capture::tests`
    passed: 23 tests.
- Capture module root conversion on 2026-06-12:
  - Moved the parent module from `recorder/platform/linux/capture.rs` to
    `recorder/platform/linux/capture/mod.rs` so further ownership files can
    live under `capture/` directly.
  - `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
- Sampler split on 2026-06-12:
  - Added `recorder/platform/linux/capture/sampler.rs` for
    `LatestPipeWireFrameSampler`, `PipeWireCapturedFrame`, shared sampler
    ownership, latest-frame log summaries, scheduled-frame log summaries, and
    frame interval calculation.
  - Current Linux capture line counts: `capture/mod.rs` 2,101,
    `capture/preflight.rs` 250, `capture/sampler.rs` 243.
  - `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings.
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml recorder::platform::linux::capture::tests`
    passed: 23 tests.
  - Remaining Linux capture suppressions:
    `#![allow(dead_code)]` and two `#[allow(clippy::too_many_arguments)]`
    entries in `capture/mod.rs`.
- Scheduler, frame conversion, preview, and session splits on 2026-06-12:
  - Added `capture/scheduler.rs` for scheduler timing, shutdown polling,
    latest-frame snapshotting, DMA-BUF materialization, composition, benchmark
    stages, writer backpressure, and scheduled-frame logging.
  - Added `capture/frame_convert.rs` for PipeWire CPU-readable and DMA-BUF
    frame extraction, frame-format mapping, storage mapping, DRM fourcc mapping,
    and structured `DmaBufFrameInput` / `CpuReadableFrameInput` arguments.
  - Added `capture/preview.rs` for preview stream setup, preview frame
    cropping/encoding, per-scene preview throttling, and Tauri preview events.
  - Added `capture/session.rs` for capture/preview session structs, shutdown
    control, startup readiness/error notification, stream config mapping,
    PipeWire capture loop setup, and negotiated format logging.
  - Removed the broad `#![allow(dead_code)]`; this exposed stale sampler
    cadence state, which was deleted because scheduler timing now owns cadence.
  - Current Linux capture line counts: `capture/mod.rs` 908,
    `capture/frame_convert.rs` 246, `capture/preflight.rs` 250,
    `capture/preview.rs` 215, `capture/sampler.rs` 221,
    `capture/scheduler.rs` 320, `capture/session.rs` 445.
  - `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml` passed
    without warnings.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings.
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml recorder::platform::linux::capture::tests`
    passed: 23 tests.
  - Remaining Linux capture cleanup: move inline tests into `capture/tests.rs`
    and run full Rust verification.
- Final Linux capture cleanup on 2026-06-12:
  - Moved the inline Linux capture tests into `capture/tests.rs`.
  - Current Linux capture line counts: `capture/mod.rs` 166,
    `capture/frame_convert.rs` 246, `capture/preflight.rs` 250,
    `capture/preview.rs` 215, `capture/sampler.rs` 221,
    `capture/scheduler.rs` 320, `capture/session.rs` 445,
    `capture/tests.rs` 740.
  - `rg "allow\\(|too_many_arguments|dead_code" apps/desktop/src-tauri/src/recorder/platform/linux/capture`
    returned no matches.
  - `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings.
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` failed under
    the default sandbox because loopback-binding transcription tests hit
    `Operation not permitted`.
  - The same `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml`
    passed outside the sandbox: 277 tests.

### 4. Split library command ownership

Source: `apps/desktop/src-tauri/src/library.rs`

Current state:

- Converted to `apps/desktop/src-tauri/src/library/`.
- `library/mod.rs` is down to `178` lines and owns module wiring plus small
  shared filesystem/time/title/delete helpers.
- DTOs, commands, redactions, debug export, playback asset helpers,
  thumbnails, row mapping, waveforms, and tests now have separate files.
- No broad lint/dead-code suppressions remain in `library/`.

Tasks:

- [x] Create `apps/desktop/src-tauri/src/library/mod.rs`.
- [x] Move serialized library DTOs into `library/types.rs`.
- [x] Move Tauri command entrypoints into `library/commands.rs`.
- [x] Move normalize/create/delete/unredact logic into `library/redactions.rs`.
- [x] Move playback session and asset freshness logic into `library/playback.rs`.
- [x] Move thumbnail generation into `library/thumbnails.rs`.
- [x] Move debug-data export behind settings checks into `library/debug.rs`.
- [x] Move row-to-DTO conversion into `library/mapping.rs`.
- [x] Keep existing `library/waveforms.rs`.
- [x] Keep `lib.rs` invoke handler command names unchanged.
- [x] Move existing library tests beside the modules they cover or into
      `library/tests.rs`.
- [x] Verify with `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- [x] Verify with `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- [x] Verify with `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`.
- [x] Verify with `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml`.

Verification notes:

- Library ownership split on 2026-06-12:
  - Moved `library.rs` to `library/mod.rs`.
  - Added `library/types.rs` for serialized request/response DTOs.
  - Added `library/commands.rs` for Tauri command entrypoints; `lib.rs`
    invoke handler paths now point at `library::commands::*` while command
    names stay unchanged through existing `#[tauri::command(rename = ...)]`
    attributes.
  - Added `library/redactions.rs` for redaction interval normalization,
    unredaction subtraction, track-id normalization, and redaction eligibility.
  - Added `library/debug.rs` for the debug-tools settings gate and sanitized
    debug row JSON export.
  - Added `library/playback.rs` for playback asset freshness, temp output
    paths, and atomic FFmpeg asset writes.
  - Added `library/thumbnails.rs` for overview thumbnail generation.
  - Added `library/mapping.rs` for summary/detail DTO mapping, segment/track
    DTOs, timeline duration, and probe parsing.
  - Moved inline tests to `library/tests.rs`.
  - Current library line counts: `commands.rs` 338, `debug.rs` 214,
    `mapping.rs` 306, `mod.rs` 178, `playback.rs` 67,
    `redactions.rs` 310, `tests.rs` 543, `thumbnails.rs` 75,
    `types.rs` 295, `waveforms.rs` 715.
  - `rg "allow\\(|too_many_arguments|dead_code" apps/desktop/src-tauri/src/library`
    returned no matches.
  - `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings.
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml library::tests`
    passed: 23 tests.
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` passed
    outside the sandbox: 277 tests.

### 5. Split AI settings UI composition

Source: `apps/desktop/src/features/settings/components/ai-settings-tab.tsx`

Current problems:

- `796` lines in essentially one React component.
- `corepack yarn lint` currently fails on Prettier errors in this file.
- The component repeats deep `AppSettings` update spreads and mixes
  transcription, vision, model-cache, and processing controls.

Tasks:

- [x] Keep `AiSettingsTab` as a thin composition component.
- [x] Split transcription settings into a focused card component.
- [x] Split vision settings into a focused card component.
- [x] Keep model-cache behavior in `AiModelCacheCard` or a focused wrapper.
- [x] Reuse and finish `AiActivityProcessingCard` for activity and processing
      controls.
- [x] Add typed draft-update helpers to avoid repeated deep object spreads.
- [x] Reuse existing shadcn components from `apps/desktop/src/components/ui`.
- [x] Fix the current Prettier errors.
- [x] Keep AI settings user-facing labels and privacy warning behavior intact.
- [x] Verify with `corepack yarn lint:fix`.
- [x] Verify with `corepack yarn lint`.
- [x] Verify with `corepack yarn typecheck`.
- [x] Verify with `corepack yarn test`.

Verification notes:

- `AiSettingsTab` is now 77 lines and composes focused transcription, model
  cache, vision, and activity/processing cards.
- Added `settings-draft-updaters.ts`, `ai-transcription-card.tsx`, and
  `ai-vision-card.tsx`.
- `corepack yarn lint:fix` passed.
- `corepack yarn lint` passed.
- `corepack yarn typecheck` passed.
- `corepack yarn workspace @daydream/desktop test SettingsScreen.browser.test.tsx`
  passed with loopback binding permission: 1 file, 13 tests.
- `corepack yarn test` passed with loopback binding permission: desktop 28
  files/263 tests, shared 1 file/9 tests, ffmpeg 1 file/1 test.
- The same browser-test commands failed under the default sandbox with
  `listen EPERM` before test collection, so loopback binding permission was
  required for Vitest browser tests.

### 6. Clean recorder runtime/compositor suppressions

Sources:

- `apps/desktop/src-tauri/src/recorder/capture_runtime.rs`
- `apps/desktop/src-tauri/src/recorder/compositor.rs`

Current state:

- `capture_runtime.rs` is `761` lines after stale runtime adapters, generic
  samplers, parser helpers, and unused YUV conversion helpers were removed.
- `compositor.rs` is `609` lines after unused duplicate YUV conversion helpers
  were removed.
- The previous top-level `#![allow(dead_code)]` suppressions and the
  `#[allow(clippy::too_many_arguments)]` constructor suppression are gone.

Tasks:

- [x] Remove top-level `#![allow(dead_code)]` from `capture_runtime.rs`.
- [x] Replace the `too_many_arguments` constructor/API with a small input
      struct or builder that makes ownership explicit.
- [x] Delete stale runtime APIs exposed only because of the broad allow, or move
      genuinely test-only helpers behind `#[cfg(test)]`.
- [x] Remove top-level `#![allow(dead_code)]` from `compositor.rs`.
- [x] Delete stale compositor APIs exposed only because of the broad allow, or
      move genuinely test-only helpers behind `#[cfg(test)]`.
- [x] Verify with `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- [x] Verify with `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- [x] Verify with `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`.
- [x] Verify with `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml recorder::capture_runtime::tests`.
- [x] Verify with `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml recorder::compositor::tests`.

Verification notes:

- Runtime/compositor suppression cleanup on 2026-06-12:
  - Added `CapturedDmaBufFrameInput` and changed `CapturedDmaBufFrame::new`
    to accept that input struct.
  - Updated Linux DMA-BUF capture conversion, sampler cloning, and tests to use
    the structured input.
  - Removed stale `CaptureAdapter`, generic `LatestFrameSampler`,
    `LatestDmaBufFrameSampler`, unused frame-format parsers, unused PipeWire
    storage parser, unused `into_yuv420p`, and the old capture-runtime YUV
    conversion helpers.
  - Removed duplicate unused compositor YUV conversion helpers.
  - Kept `FrameReceiver::recv` behind `#[cfg(test)]` because production uses
    `recv_timeout`.
  - `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings.
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml recorder::capture_runtime::tests`
    passed: 6 tests.
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml recorder::compositor::tests`
    passed: 7 tests.
  - Global suppression search now reports only narrow dead-code allows in
    `recorder/errors.rs`, `whisper_backend.rs`, and Linux portal code.

### 7. Split media writer ownership

Source: `apps/desktop/src-tauri/src/recorder/media_writer/`

Current problems:

- `media_writer.rs` is `1,889` lines.
- It mixes public segment/probe DTOs, GStreamer health/capability detection,
  segment lifecycle, launch-description assembly, audio EOS/cutoff handling,
  frame pumping, raw-frame conversion/repacking, media probing, utility
  formatting, and tests.
- It is runtime-critical: media finalization, stop cutoffs, stream indexes, and
  low-latency writer behavior should be easy to reason about independently.

Tasks:

- [x] Create `apps/desktop/src-tauri/src/recorder/media_writer/mod.rs` as the
      public module entry.
- [x] Move public segment/probe DTOs into `media_writer/types.rs`.
- [x] Move health/capability detection and package hints into
      `media_writer/capabilities.rs`.
- [x] Move segment start/stop lifecycle and benchmark setup into
      `media_writer/session.rs`.
- [x] Move GStreamer launch-description and audio branch assembly into
      `media_writer/pipeline.rs`.
- [x] Move audio EOS/cutoff probe handling into `media_writer/audio.rs`.
- [x] Move composed-frame pump, stop draining, PTS calculation, and raw frame
      pushes into `media_writer/frame_pump.rs`.
- [x] Move frame conversion/repacking/caps helpers into
      `media_writer/video_frame.rs`.
- [x] Move segment probing/discovery into `media_writer/probe.rs`.
- [x] Move existing media writer tests into `media_writer/tests.rs`.
- [x] Keep public function names used by recorder manager unchanged.
- [x] Verify with `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- [x] Verify with `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- [x] Verify with `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`.
- [x] Verify with `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml recorder::media_writer::tests`.

Verification notes:

- Completed on 2026-06-12:
  - Replaced flat `media_writer.rs` with a directory module.
  - Split ownership into `types.rs`, `capabilities.rs`, `session.rs`,
    `pipeline.rs`, `audio.rs`, `frame_pump.rs`, `video_frame.rs`, `probe.rs`,
    and `tests.rs`.
  - Removed parent wildcard import dependencies from the split modules where
    the session extraction exposed hidden coupling.
  - Current media writer line counts: `mod.rs` 65, `audio.rs` 143,
    `capabilities.rs` 240, `frame_pump.rs` 306, `pipeline.rs` 142,
    `probe.rs` 135, `session.rs` 307, `tests.rs` 384, `types.rs` 69,
    `video_frame.rs` 192.
  - `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings.
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml recorder::media_writer::tests`
    passed: 18 tests.

### 8. Split AI storage persistence ownership

Source: `apps/desktop/src-tauri/src/storage/ai/`

Current problems:

- `storage/ai.rs` is `1,810` lines after the latest discovery pass.
- It was a broad persistence module for model assets, recording tracks, AI job
  state, chunk state, transcript rows, activity rows, visual rows, capture
  events, and row mapping.
- Storage code is a privacy-sensitive boundary. It should make deletion,
  provenance, and structured model output persistence easy to audit without
  mixing unrelated row mappers and query families.

Tasks:

- [x] Inspect the current function groups and call sites before editing.
- [x] Create `apps/desktop/src-tauri/src/storage/ai/mod.rs` as the public
      storage entry.
- [x] Move public DTOs and row structs into `storage/ai/types.rs`.
- [x] Move shared row helpers into `storage/ai/rows.rs`.
- [x] Move model asset persistence into `storage/ai/models.rs`.
- [x] Move recording track persistence into `storage/ai/tracks.rs`.
- [x] Move processing job persistence into `storage/ai/jobs.rs`.
- [x] Move processing chunk, capture event, chunk output cleanup, and retry
      persistence into `storage/ai/chunks.rs`.
- [x] Move transcript persistence and row mapping into
      `storage/ai/transcripts.rs`.
- [x] Move activity persistence and row mapping into `storage/ai/activity.rs`.
- [x] Move visual analysis persistence and row mapping into
      `storage/ai/visual.rs`.
- [x] Confirm annotation/task-candidate and read-side aggregation modules are
      not current responsibilities of this file; do not create empty modules.
- [x] Keep existing public storage function names and SQLite schema unchanged.
- [x] Move or add focused tests beside the modules they protect when the split
      exposes behavior that is not already covered by `storage/tests.rs`.
- [x] Verify with `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- [x] Verify with `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- [x] Verify with `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`.
- [x] Verify with focused storage/AI tests before broader cargo tests.

Verification notes:

- Completed on 2026-06-12:
  - Replaced flat `storage/ai.rs` with a directory module.
  - Split ownership into `types.rs`, `rows.rs`, `models.rs`, `tracks.rs`,
    `jobs.rs`, `chunks.rs`, `transcripts.rs`, `activity.rs`, and `visual.rs`.
  - Kept public re-exports through `storage::ai`/`storage.rs`; no SQLite schema
    or public function names changed.
  - Current storage AI line counts: `mod.rs` 18, `activity.rs` 66,
    `chunks.rs` 475, `jobs.rs` 195, `models.rs` 139, `rows.rs` 170,
    `tracks.rs` 130, `transcripts.rs` 163, `types.rs` 301, `visual.rs` 206.
  - `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings.
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml storage::tests`
    passed: 42 tests.

### 9. Split settings ownership

Source: `apps/desktop/src-tauri/src/settings.rs`

Current problems:

- `settings.rs` is `1,539` lines after the storage AI split.
- It mixes Tauri command entrypoints, public settings DTOs, constants/defaults,
  limits, storage row mapping, validation, cleanup enforcement, filesystem path
  resolution, and error construction.
- Settings are a cross-feature control plane. Validation and persistence
  mapping should be isolated so future settings do not turn this into another
  monolithic boundary.

Tasks:

- [x] Create `apps/desktop/src-tauri/src/settings/mod.rs` as the public module
      entry.
- [x] Move public DTOs, command inputs, and `AppSettingsError` into
      `settings/types.rs`.
- [x] Move constants, defaults, and limits into `settings/defaults.rs`.
- [x] Move Tauri command entrypoints into `settings/commands.rs`.
- [x] Move load/save/reset state orchestration and cleanup enforcement into
      `settings/service.rs`.
- [x] Move storage row mapping and storage input serialization into
      `settings/persistence.rs`.
- [x] Move validation helpers into `settings/validation.rs`.
- [x] Keep Tauri command names, payload shapes, storage keys, and default
      values unchanged.
- [x] Verify with `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- [x] Verify with `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- [x] Verify with `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`.
- [x] Verify with focused settings/storage tests before broader cargo tests.

Verification notes:

- Completed on 2026-06-12:
  - Replaced flat `settings.rs` with a directory module.
  - Split ownership into `types.rs`, `defaults.rs`, `commands.rs`,
    `service.rs`, `persistence.rs`, `validation.rs`, and `tests.rs`.
  - Updated `lib.rs` Tauri handler paths to `settings::commands::*`; external
    command names remain unchanged through existing `#[tauri::command(rename =
    "...")]` attributes.
  - Current settings line counts: `mod.rs` 13, `commands.rs` 71,
    `defaults.rs` 171, `persistence.rs` 385, `service.rs` 91, `tests.rs` 342,
    `types.rs` 222, `validation.rs` 292.
  - `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings.
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml settings::tests`
    passed: 5 tests.

### 10. Split visual AI processing ownership

Source: `apps/desktop/src-tauri/src/ai/processing/visual/`

Current problems:

- `visual.rs` is `1,284` lines after the settings split.
- It mixes chunk orchestration, remote vision-provider config, FFmpeg frame
  extraction, PNG decoding/cropping, image-delta region detection, Tesseract
  OCR parsing, VLM HTTP request/response handling, and persistence mapping.
- This is privacy-sensitive AI processing over screen content. Source frame
  references, crop provenance, and persisted OCR/VLM output should be isolated
  enough to audit without reading unrelated image-processing code.

Tasks:

- [x] Inspect current visual helper groups and test coverage before editing.
- [x] Create `apps/desktop/src-tauri/src/ai/processing/visual/mod.rs` as the
      orchestration entry.
- [x] Move public/internal DTOs into `visual/types.rs`.
- [x] Move frame extraction and source-frame reference helpers into
      `visual/extraction.rs`.
- [x] Move PNG decode, crop writing, image delta, tile merge, and region clamp
      helpers into `visual/image.rs`.
- [x] Move Tesseract OCR execution and TSV parsing into `visual/ocr.rs`.
- [x] Move VLM provider config, request construction, HTTP call, and response
      parsing into `visual/vlm.rs`.
- [x] Move output-to-storage input mapping and persistence into
      `visual/persistence.rs`.
- [x] Keep chunk behavior, source-frame references, persisted rows, and
      cancellation checks unchanged.
- [x] Verify with `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- [x] Verify with `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- [x] Verify with `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`.
- [x] Verify with focused AI visual tests before broader cargo tests.

Verification notes:

- Completed on 2026-06-12:
  - Replaced flat `visual.rs` with a directory module.
  - Split ownership into `types.rs`, `extraction.rs`, `image.rs`, `ocr.rs`,
    `vlm.rs`, and `persistence.rs`.
  - Kept processing re-exports for existing AI tests and callers.
  - Current visual processing line counts: `mod.rs` 126, `extraction.rs` 250,
    `image.rs` 298, `ocr.rs` 134, `persistence.rs` 111, `types.rs` 110,
    `vlm.rs` 302.
  - `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings.
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml ai::tests`
    passed: 44 tests.

### 11. Split AI processing worker ownership

Source: `apps/desktop/src-tauri/src/ai/processing/worker/`

Current problems:

- The old flat `worker.rs` was `1,244` lines after the visual processing
  split.
- The split module is now explicit by ownership: `mod.rs` 16 lines,
  `scheduler.rs` 136, `enqueue.rs` 383, `execution.rs` 474,
  `completion.rs` 157, `actions.rs` 75, `cancel.rs` 26, and `types.rs` 4.
- It mixes stale-job recovery, worker tick scheduling, job enqueue/planning
  readiness, chunk claiming/execution, completion summarization, job action
  updates, retry/cancel behavior, notification persistence, and cancel monitor
  spawning.
- Processing worker behavior is lifecycle-critical and privacy-sensitive. Job
  state transitions and notification side effects should be isolated from
  source-specific chunk execution.

Tasks:

- [x] Inspect current worker function groups and test coverage before editing.
- [x] Create `apps/desktop/src-tauri/src/ai/processing/worker/mod.rs` as the
      public worker entry.
- [x] Move worker-facing DTOs and small context structs into
      `worker/types.rs`.
- [x] Move stale-job recovery and worker tick scheduling into
      `worker/scheduler.rs`.
- [x] Move enqueue, readiness checks, and selected transcription model logic
      into `worker/enqueue.rs`.
- [x] Move chunk claiming/execution and per-source dispatch into
      `worker/execution.rs`.
- [x] Move job completion/failure summarization and final state updates into
      `worker/completion.rs`.
- [x] Move pause/resume/cancel/retry action handling into `worker/actions.rs`.
- [x] Move cancel monitor spawning into `worker/cancel.rs`.
- [x] Keep job state transitions, notification payloads, event emission, and
      chunk status/result semantics unchanged.
- [x] Verify with `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- [x] Verify with `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- [x] Verify with `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`.
- [x] Verify with focused AI worker tests before broader cargo tests.

Verification notes:

- Completed on 2026-06-12:
  - Replaced flat `ai/processing/worker.rs` with a directory module.
  - Split scheduler/recovery, enqueue/readiness, execution dispatch,
    completion/failure, action updates, cancel monitor, and shared DTO
    ownership into separate files.
  - Kept existing processing-state event emission, notification payloads, chunk
    status/result handling, and Tauri command surfaces unchanged.
  - Current worker line counts: `mod.rs` 16, `scheduler.rs` 136,
    `enqueue.rs` 383, `execution.rs` 474, `completion.rs` 157,
    `actions.rs` 75, `cancel.rs` 26, `types.rs` 4.
  - `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings.
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml ai::tests`
    passed: 44 tests.
  - `COREPACK_HOME=/tmp/corepack corepack yarn lint` passed.
  - Full `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` failed
    under the default sandbox only because 9 transcription tests could not bind
    loopback ports, then passed with loopback permission: 276 tests.

### 12. Split playback server ownership

Source: `apps/desktop/src-tauri/src/playback_server/`

Current problems:

- The old flat `playback_server.rs` was `1,144` lines after the worker split.
- The split module is now explicit by ownership: `mod.rs` 191 lines,
  `http.rs` 303, `thumbnails.rs` 277, `encoders.rs` 144,
  `ffmpeg_args.rs` 42, and `tests.rs` 245.
- It mixes request parsing, session-token validation, HTTP response building,
  stream argument construction, thumbnail planning/cache/queue state, FFmpeg
  process execution, playback encoder selection, and tests.
- Playback serves sensitive local recording media. The auth/session boundary,
  FFmpeg command construction, and thumbnail cache state should be isolated so
  future timeline playback and redaction behavior can be changed without
  touching HTTP parsing code.

Tasks:

- [x] Inspect current playback server function groups and tests before editing.
- [x] Create `apps/desktop/src-tauri/src/playback_server/mod.rs` as the public
      module entry.
- [x] Move request parsing, token validation, and response helpers into
      `playback_server/http.rs`.
- [x] Move stream and thumbnail FFmpeg argument construction into
      `playback_server/ffmpeg_args.rs`.
- [x] Move thumbnail cache, queue, and session cleanup state into
      `playback_server/thumbnails.rs`.
- [x] Move encoder capability/probe helpers into
      `playback_server/encoders.rs`.
- [x] Keep playback URLs, token validation behavior, cache semantics, and
      privacy-safe error responses unchanged.
- [x] Move or preserve playback server tests beside the module they cover.
- [x] Verify with `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- [x] Verify with `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- [x] Verify with `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`.
- [x] Verify with focused playback server tests before broader cargo tests.

Verification notes:

- Completed on 2026-06-12:
  - Replaced flat `playback_server.rs` with a directory module.
  - Split request parsing/session validation/response streaming, FFmpeg
    argument construction, thumbnail queue/cache/workers, encoder probing, and
    playback tests into separate files.
  - Kept playback URLs, token validation behavior, thumbnail cache/session
    cleanup semantics, FFmpeg output streaming, and privacy-safe response
    bodies unchanged.
  - Current playback server line counts: `mod.rs` 191, `http.rs` 303,
    `thumbnails.rs` 277, `encoders.rs` 144, `ffmpeg_args.rs` 42,
    `tests.rs` 245.
  - `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml playback_server::tests`
    passed: 12 tests.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings.
  - `COREPACK_HOME=/tmp/corepack corepack yarn lint` passed.
  - Full `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` failed
    under the default sandbox only because 9 transcription tests could not bind
    loopback ports, then passed with loopback permission: 276 tests.

### 13. Split AI model/download ownership

Source: `apps/desktop/src-tauri/src/ai/models/`

Current problems:

- The old flat `ai/models.rs` was `1,045` lines after the playback split.
- The split module is now explicit by ownership: `mod.rs` 25 lines,
  `catalog.rs` 214, `download.rs` 469, `manager.rs` 81,
  `paths.rs` 78, and `recovery.rs` 227.
- It mixes model catalog metadata, local cache path construction, database
  state mapping, resumable download transport, checksum validation, recovery
  decisions, cancellation, notification/event emission, and self-test behavior.
- Model download state is a privacy and reliability boundary. Recovery,
  persisted asset state, and notification side effects should be isolated from
  transport details so future local model providers or packaged model assets do
  not touch every path.

Tasks:

- [x] Inspect current model manager function groups and tests before editing.
- [x] Create `apps/desktop/src-tauri/src/ai/models/mod.rs` as the public model
      manager entry.
- [x] Move static model catalog and metadata DTO mapping into
      `models/catalog.rs`.
- [x] Move cache path and partial/final file helpers into `models/paths.rs`.
- [x] Move download request execution, range handling, and progress accounting
      into `models/download.rs`.
- [x] Move checksum validation and recovery decision logic into
      `models/recovery.rs`.
- [x] Evaluate notification/event helper ownership. Kept notification writes
      with their owning catalog/download/recovery flows because a separate
      module would add indirection without reducing coupling.
- [x] Keep model IDs, download URLs, checksum behavior, cancellation semantics,
      persisted asset rows, and event names unchanged.
- [x] Preserve existing AI model/download tests.
- [x] Verify with `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- [x] Verify with `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- [x] Verify with `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`.
- [x] Verify with focused AI model/download tests before broader cargo tests.

Verification notes:

- Completed on 2026-06-12:
  - Replaced flat `ai/models.rs` with a directory module.
  - Split model manager queue state, static catalog and database DTO mapping,
    filesystem cache/checksum helpers, resumable download execution and
    finalization, and recovery decisions into separate files.
  - Kept model IDs, Hugging Face source URLs, pinned SHA-1 checksums,
    cancellation behavior, persisted asset rows, notification payloads, and
    `ai-models-changed` event emission unchanged.
  - Current AI model line counts: `mod.rs` 25, `catalog.rs` 214,
    `download.rs` 469, `manager.rs` 81, `paths.rs` 78,
    `recovery.rs` 227.
  - `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml ai::tests`
    passed: 44 tests.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings.
  - `COREPACK_HOME=/tmp/corepack corepack yarn lint` passed.
  - Full `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` failed
    under the default sandbox only because 9 transcription tests could not bind
    loopback ports, then passed with loopback permission: 276 tests.

### 14. Split storage schema ownership

Source: `apps/desktop/src-tauri/src/storage/schema/`

Current problems:

- The old flat `storage/schema.rs` was `864` lines after the AI model split.
- The split module is now explicit by ownership: `mod.rs` 7 lines,
  `connection.rs` 18, `tables.rs` 21, `migrations.rs` 22,
  `migrations/initial.rs` 78, `migrations/scene_recording.rs` 283,
  `migrations/settings_processing.rs` 304, and
  `migrations/ai_outputs.rs` 173.
- It mixes connection opening, SQLite pragma setup, core table lists, and every
  embedded migration batch in one file.
- SQLite is the durable local metadata boundary for sensitive recordings and
  derived AI data. Migration SQL should stay explicit, but connection setup,
  table constants, and migration batches should be easier to review
  independently.

Tasks:

- [x] Inspect current schema migration batches and storage tests before
      editing.
- [x] Create `apps/desktop/src-tauri/src/storage/schema/mod.rs` as the public
      schema entry.
- [x] Move connection opening and pragma setup into `schema/connection.rs`.
- [x] Move core table constants into `schema/tables.rs`.
- [x] Move embedded migration batches into `schema/migrations.rs`, preserving
      order exactly.
- [x] Split large migration batches into ordered `schema/migrations/*.rs`
      modules so no new large migration file remains.
- [x] Keep schema versioning, migration SQL, foreign key behavior, and journal
      pragmas unchanged.
- [x] Preserve existing storage migration tests.
- [x] Verify with `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- [x] Verify with `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- [x] Verify with `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`.
- [x] Verify with focused storage schema tests before broader cargo tests.

Verification notes:

- Completed on 2026-06-12:
  - Replaced flat `storage/schema.rs` with a directory module.
  - Split connection opening/pragmas, core table constants, migration ordering,
    and migration SQL batches into separate files.
  - Kept migration order, SQL text, schema version behavior, SQLite pragmas,
    and diagnostics table constants unchanged.
  - Current schema line counts: `mod.rs` 7, `connection.rs` 18,
    `tables.rs` 21, `migrations.rs` 22, `migrations/initial.rs` 78,
    `migrations/scene_recording.rs` 283,
    `migrations/settings_processing.rs` 304,
    `migrations/ai_outputs.rs` 173.
  - `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml storage::tests`
    passed: 42 tests.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings.
  - `COREPACK_HOME=/tmp/corepack corepack yarn lint` passed.
  - Full `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` failed
    under the default sandbox only because 9 transcription tests could not bind
    loopback ports, then passed with loopback permission: 276 tests.

### 15. Split AI results/read-side ownership

Source: `apps/desktop/src-tauri/src/ai/results/`

Current problems:

- The old flat `ai/results.rs` was `818` lines after the schema split.
- The split module is now explicit by ownership: `mod.rs` 17 lines,
  `activity.rs` 31, `processing.rs` 157, `transcript.rs` 312,
  `visual.rs` 243, and `vlm.rs` 85.
- It mixes transcript read-side queries and redaction filtering, processing
  state composition, transcription track state derivation, activity interval
  mapping, visual OCR/VLM redaction filtering, VLM provider state, processing
  job DTO mapping, and processing-state event emission.
- This read side is privacy-sensitive because it decides what derived
  transcript, OCR, VLM, and activity data remains visible after redaction.
  Source-specific visibility and DTO mapping should be isolated so future
  timeline/search features do not accidentally bypass redaction filters.

Tasks:

- [x] Inspect current read-side function groups and AI tests before editing.
- [x] Create `apps/desktop/src-tauri/src/ai/results/mod.rs` as the public
      results entry.
- [x] Move transcript queries, redaction filtering, word mapping, and
      transcription track state into `results/transcript.rs`.
- [x] Move activity interval read-side mapping into `results/activity.rs`.
- [x] Move visual OCR/VLM queries, redaction visibility, and visual region
      mapping into `results/visual.rs`.
- [x] Move VLM provider/chunk state mapping into `results/vlm.rs` if it is not
      already cleanly owned by `visual.rs`.
- [x] Move generic processing job/chunk DTO mapping, status counts, and event
      emission into `results/processing.rs`.
- [x] Keep redaction filtering semantics, DTO shapes, event names, and Tauri
      command behavior unchanged.
- [x] Preserve existing AI read-side tests.
- [x] Verify with `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- [x] Verify with `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- [x] Verify with `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`.
- [x] Verify with focused AI read-side tests before broader cargo tests.

Verification notes:

- Completed on 2026-06-12:
  - Replaced flat `ai/results.rs` with a directory module.
  - Split transcript read-side and track state, activity mapping, visual
    OCR/VLM redaction filtering, VLM state mapping, and generic processing DTO
    mapping/event emission into separate files.
  - Kept redaction filtering semantics, DTO shapes, event names, and Tauri
    command behavior unchanged.
  - Current AI results line counts: `mod.rs` 17, `activity.rs` 31,
    `processing.rs` 157, `transcript.rs` 312, `visual.rs` 243,
    `vlm.rs` 85.
  - `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml ai::tests`
    passed: 44 tests.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings.
  - `COREPACK_HOME=/tmp/corepack corepack yarn lint` passed.
  - Full `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` failed
    under the default sandbox only because 9 transcription tests could not bind
    loopback ports, then passed with loopback permission: 276 tests.

### 16. Split Whisper backend ownership

Source: `apps/desktop/src-tauri/src/whisper_backend.rs`

Current problems:

- `whisper_backend.rs` is `812` lines after the AI results split and is now the
  largest production Rust source file outside tests.
- It mixes backend availability reporting, model load/unload lifecycle,
  transcription execution, whisper parameter construction, language catalog
  mapping, token/word extraction, silence detection, WAV parsing, and tests.
- It also has narrow `#[allow(dead_code)]` suppressions for model lifecycle and
  transcription functions that are integration-bound today. Those should be
  removed or moved behind clearer ownership boundaries where possible.

Tasks:

- [x] Inspect current Whisper backend function groups and tests before editing.
- [x] Create `apps/desktop/src-tauri/src/whisper_backend/mod.rs` as the public
      backend entry.
- [x] Move backend status and execution backend selection into
      `whisper_backend/backend.rs`.
- [x] Move model load/unload lifecycle into `whisper_backend/model.rs`.
- [x] Move transcription execution, parameter construction, token/word
      extraction, and empty/cancelled responses into
      `whisper_backend/transcribe.rs`.
- [x] Move supported language mapping and validation into
      `whisper_backend/languages.rs`.
- [x] Move silence gate and speech range detection into
      `whisper_backend/silence.rs`.
- [x] Move WAV parsing into `whisper_backend/wav.rs`.
- [x] Remove or narrowly justify existing dead-code suppressions.
- [x] Keep public API, error codes, backend selection behavior, silence-gate
      math, WAV validation, and transcription output shape unchanged.
- [x] Preserve existing Whisper backend tests.
- [x] Verify with `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- [x] Verify with `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- [x] Verify with `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`.
- [x] Verify with focused Whisper backend tests before broader cargo tests.

Verification notes:

- Completed on 2026-06-12:
  - Replaced flat `whisper_backend.rs` with `whisper_backend/`.
  - Added `backend.rs` for backend selection/status and context params,
    `model.rs` for loaded model lifecycle, `transcribe.rs` for full
    transcription execution and token extraction, `languages.rs` for Whisper
    language lookup, `silence.rs` for speech range detection, `wav.rs` for WAV
    parsing, and `tests.rs` for the existing unit tests.
  - Kept the root module as the compatibility boundary for existing
    `crate::whisper_backend::*` callers.
  - Current line counts: `backend.rs` 45, `languages.rs` 60, `mod.rs` 110,
    `model.rs` 115, `silence.rs` 108, `transcribe.rs` 193, `wav.rs` 71,
    `tests.rs` 202.
  - Existing dead-code suppressions are now narrow and documented around
    integration hooks: model load self-test, one-shot transcription, no-state
    model probe, and generated whisper.cpp bindings.
  - `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml whisper_backend::tests`
    passed: 10 tests.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings.
  - `COREPACK_HOME=/tmp/corepack corepack yarn lint` passed.
  - Full `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` failed
    under the default sandbox only because 9 transcription tests could not bind
    loopback ports; the same command passed with loopback permission: 276
    tests.

### 17. Split capture runtime ownership

Source: `apps/desktop/src-tauri/src/recorder/capture_runtime.rs`

Current problems:

- `capture_runtime.rs` is now the largest production Rust source file at 761
  lines after the Whisper split.
- It mixes runtime source identity, frame format/storage enums, CPU frame
  validation, DMA-BUF frame validation, writer handoff planning, privacy-safe
  debug formatting, bounded frame channel send/receive behavior, worker thread
  joining, timing counters, and tests.
- The module is on the critical screen-recording privacy boundary. Debug
  formatting and error messages must continue to avoid raw frame bytes and
  captured content.

Tasks:

- [x] Inspect current capture runtime type groups, callers, and tests before
      editing.
- [x] Create `apps/desktop/src-tauri/src/recorder/capture_runtime/mod.rs` as
      the public recorder runtime entry.
- [x] Move source identity types into `capture_runtime/sources.rs`.
- [x] Move frame format/storage enums and writer handoff planning into
      `capture_runtime/formats.rs`.
- [x] Move CPU-readable captured frame construction, validation, and debug
      formatting into `capture_runtime/cpu_frame.rs`.
- [x] Move DMA-BUF frame construction, validation, fd ownership, and debug
      formatting into `capture_runtime/dmabuf_frame.rs`.
- [x] Move bounded frame sender/receiver channel behavior into
      `capture_runtime/channel.rs`.
- [x] Move `RuntimeWorker` thread joining and cleanup error mapping into
      `capture_runtime/worker.rs`.
- [x] Keep `SharedCaptureTiming` and timing counter updates behind a narrow
      runtime-owned API.
- [x] Preserve existing public recorder-internal type paths through root
      re-exports unless callers are moved in the same patch.
- [x] Preserve privacy-safe debug output and existing error codes/messages.
- [x] Move existing capture runtime tests to `capture_runtime/tests.rs` or
      colocate them with the module they protect.
- [x] Verify with `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- [x] Verify with `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- [x] Verify with `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml recorder::capture_runtime::tests`.
- [x] Verify with `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`.
- [x] Verify with `COREPACK_HOME=/tmp/corepack corepack yarn lint`.

Verification notes:

- Completed on 2026-06-12:
  - Replaced flat `capture_runtime.rs` with `capture_runtime/`.
  - Added `sources.rs` for runtime source identity, `formats.rs` for frame
    format/storage policy, `cpu_frame.rs` for CPU-readable frame validation and
    privacy-safe debug formatting, `dmabuf_frame.rs` for DMA-BUF frame
    validation and fd debug formatting, `channel.rs` for bounded frame channel
    behavior, `worker.rs` for capture worker joining, `timing.rs` for capture
    timing counters, and `tests.rs` for existing unit tests.
  - Kept root re-exports so existing recorder-internal caller paths stayed
    stable.
  - Current line counts: `channel.rs` 69, `cpu_frame.rs` 184,
    `dmabuf_frame.rs` 116, `formats.rs` 65, `mod.rs` 33, `sources.rs` 16,
    `tests.rs` 188, `timing.rs` 88, `worker.rs` 32.
  - `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml recorder::capture_runtime::tests`
    passed: 6 tests.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings.
  - `COREPACK_HOME=/tmp/corepack corepack yarn lint` passed.
  - Full `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` failed
    under the default sandbox only because 9 transcription tests could not bind
    loopback ports; the same command passed with loopback permission: 276
    tests.

### 18. Split library waveform ownership

Source: `apps/desktop/src-tauri/src/library/waveforms.rs`

Current problems:

- `waveforms.rs` is now the largest production Rust file at 715 lines after
  the capture runtime split.
- It mixes recording-level waveform DTO assembly, FFmpeg sample extraction,
  peak/RMS binning, stream max-amplitude probing, tile request normalization,
  per-segment tile generation, cache key/path construction, JSON cache
  read/write, file fingerprinting, and tests in one module.
- The module is library/read-side code, but it reaches into process execution
  and cache persistence directly. Those should be owned at narrower edges so
  waveform math remains deterministic and easy to test.

Tasks:

- [x] Inspect current waveform function groups, callers, and tests before
      editing.
- [x] Create `apps/desktop/src-tauri/src/library/waveforms/mod.rs` as the
      read-side entry.
- [x] Move FFmpeg process extraction and stream max-amplitude probing into
      `waveforms/ffmpeg_source.rs`.
- [x] Move waveform sample binning, normalization, and scale math into
      `waveforms/math.rs`.
- [x] Move tile context/input normalization and tile generation into
      `waveforms/tiles.rs`.
- [x] Move cache path/key construction, safe path components, file
      fingerprints, and JSON cache read/write into `waveforms/cache.rs`.
- [x] Keep recording-level `waveforms_for_recording` and
      `waveform_tiles_for_recording` as the root orchestration API.
- [x] Preserve current cache version, cache paths, tile bucket semantics,
      error strings, and DTO shapes.
- [x] Move existing waveform tests beside the module they protect or into
      `waveforms/tests.rs`.
- [x] Verify with `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- [x] Verify with `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- [x] Verify with `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml library::tests`.
- [x] Verify with `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`.
- [x] Verify with `COREPACK_HOME=/tmp/corepack corepack yarn lint`.

Verification notes:

- Completed on 2026-06-12:
  - Replaced flat `library/waveforms.rs` with `library/waveforms/`.
  - Added `ffmpeg_source.rs` for FFmpeg sample extraction, tile sample
    extraction, stream max-volume probing, and volumedetect parsing.
  - Added `math.rs` for peak bins, peak/RMS bins, and tile sample counts.
  - Added `tiles.rs` for tile request resolution, segment tile range
    calculation, tile cache lookup/write orchestration, and tile DTO assembly.
  - Added `cache.rs` for cache DTOs, cache paths, safe path components, file
    fingerprints, cache key hashing, and JSON cache persistence.
  - Kept root `mod.rs` as recording-level orchestration for waveforms and
    waveform tiles, with test-only re-exports for the existing library tests.
  - Current line counts: `mod.rs` 224, `tiles.rs` 168,
    `ffmpeg_source.rs` 176, `cache.rs` 147, `math.rs` 60.
  - `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml library::tests`
    passed: 23 tests.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings.
  - `COREPACK_HOME=/tmp/corepack corepack yarn lint` passed.
  - Full `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` failed
    under the default sandbox only because 9 transcription tests could not bind
    loopback ports; the same command passed with loopback permission: 276
    tests.

### 19. Split recorder manager ownership

Source: `apps/desktop/src-tauri/src/recorder/manager.rs`

Current problems:

- `manager.rs` is now the largest production Rust file at 712 lines after the
  waveform split.
- It mixes recorder manager state, active recording/preview structs, pending
  start structs, segment runtime stop/finalization, preview generation logic,
  recording lifecycle methods, status DTO construction, file path helpers, and
  storage failure mapping.
- Recorder lifecycle state is central to visible recording control. The split
  must preserve state transition behavior, backend health reporting, storage
  writes, and privacy-safe capture timing logs.

Tasks:

- [x] Inspect current recorder manager function groups, submodules, callers,
      and tests before editing.
- [x] Create `apps/desktop/src-tauri/src/recorder/manager/mod.rs` as the
      manager entry.
- [x] Move runtime state structs into `manager/runtime.rs`.
- [x] Move active recording, active segment, and pending start structs into
      `manager/recording_state.rs`.
- [x] Move segment runtime stop/finalization and capture timing logging into
      `manager/segment_runtime.rs`.
- [x] Move status construction helpers into `manager/status.rs`.
- [x] Move path/duration/file-name helpers into `manager/paths.rs` or an
      equivalent narrow utility module.
- [x] Move storage failure mapping for segment finalization into
      `manager/finalization.rs`.
- [x] Keep existing lifecycle submodules `resume`, `selection`, `start`, and
      `stop` wired through the new module root.
- [x] Preserve `RecorderManager` public behavior, state transition guards,
      event-visible statuses, and test expectations.
- [x] Verify with `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- [x] Verify with `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- [x] Verify with `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml recorder::tests`.
- [x] Verify with `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`.
- [x] Verify with `COREPACK_HOME=/tmp/corepack corepack yarn lint`.

Verification notes:

- Completed on 2026-06-12:
  - Replaced flat `recorder/manager.rs` with `recorder/manager/`.
  - Added `runtime.rs` for manager runtime state, active preview state, and
    preview start decisions.
  - Added `recording_state.rs` for active recordings, active segments, pending
    segment starts, and shared segment runtime handles.
  - Added `segment_runtime.rs` for segment stop behavior and capture timing
    logging.
  - Added `status.rs` for status DTO assembly, pending lifecycle checks, and
    unavailable backend health helpers.
  - Added `paths.rs` for duration, file-name, and segment path helpers.
  - Added `finalization.rs` for storage failure mapping during segment
    finalization.
  - Kept existing lifecycle modules `resume`, `selection`, `start`, and `stop`
    wired through the new root.
  - Current line counts: `mod.rs` 358, `status.rs` 212,
    `segment_runtime.rs` 70, `recording_state.rs` 59, `runtime.rs` 30,
    `finalization.rs` 27, and `paths.rs` 13.
  - `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml recorder::tests`
    passed: 34 tests.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings.
  - `COREPACK_HOME=/tmp/corepack corepack yarn lint` passed.
  - Full `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` failed
    under the default sandbox only because 9 transcription tests could not bind
    loopback ports; the same command passed with loopback permission: 276
    tests.

### 20. Split diagnostics ownership

Source: `apps/desktop/src-tauri/src/diagnostics.rs`

Current problems:

- `diagnostics.rs` is now the largest production Rust file at 658 lines after
  the recorder manager split.
- It mixes diagnostics command payloads, media segment probe parsing,
  transcript and processing diagnostics, portal-token metadata, media-writer
  health, storage reads, and tests.
- Diagnostics is a support surface for sensitive local capture data. The split
  must preserve privacy-safe log handling and avoid exposing raw transcript,
  OCR, prompt, or model content.

Tasks:

- [x] Inspect current diagnostics function groups, callers, command
      registration, and tests before editing.
- [x] Create `apps/desktop/src-tauri/src/diagnostics/mod.rs` as the diagnostics
      entry.
- [x] Move diagnostics DTOs and shared constants into `diagnostics/types.rs`.
- [x] Move Tauri command orchestration into `diagnostics/commands.rs` or keep
      only the command entrypoints in `mod.rs` if that better preserves handler
      registration.
- [x] Move media segment probe parsing into `diagnostics/latest_recording.rs`.
- [x] Move transcript and processing diagnostics into
      `diagnostics/ai_processing.rs`.
- [x] Keep log path, environment filter, and log tail helpers owned by
      `telemetry`; diagnostics only calls the telemetry API.
- [x] Move tests into `diagnostics/tests.rs` or focused module-local tests.
- [x] Preserve existing Tauri command names, IPC payload shapes, and test
      expectations.
- [x] Verify with `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- [x] Verify with `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- [x] Verify with `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml diagnostics::tests`.
- [x] Verify with `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`.
- [x] Verify with `COREPACK_HOME=/tmp/corepack corepack yarn lint`.

Verification notes:

- Completed on 2026-06-12:
  - Replaced flat `diagnostics.rs` with `diagnostics/`.
  - Added `commands.rs` for Tauri command entrypoints and top-level
    diagnostics orchestration.
  - Added `types.rs` for all serialized diagnostics payloads.
  - Added `ai_processing.rs` for redaction-aware AI processing, chunk, job, and
    transcript-track summaries.
  - Added `latest_recording.rs` for active/latest recording diagnostics and
    media segment probe JSON parsing.
  - Added `media_writer.rs` for recorder media-writer health mapping.
  - Added `portal.rs` for Linux screencast restore-token diagnostics.
  - Added `tests.rs` for the probe parsing and private transcript omission
    tests.
  - Kept debug log tail reading in `telemetry`; diagnostics remains only the
    command caller for that telemetry-owned behavior.
  - Updated command registration from `diagnostics::*` to
    `diagnostics::commands::*`; Tauri IPC command names are unchanged.
  - Current line counts: `ai_processing.rs` 184, `types.rs` 182,
    `latest_recording.rs` 105, `tests.rs` 83, `commands.rs` 76,
    `portal.rs` 43, `media_writer.rs` 14, and `mod.rs` 10.
  - `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml diagnostics::tests`
    passed: 2 tests.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings.
  - `COREPACK_HOME=/tmp/corepack corepack yarn lint` passed.
  - Full `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` failed
    under the default sandbox only because 9 transcription tests could not bind
    loopback ports; the same command passed with loopback permission: 276
    tests.

### 21. Split settings screen orchestration

Source: `apps/desktop/src/screens/SettingsScreen.tsx`

Current problems:

- `SettingsScreen.tsx` is now the largest production route component at 637
  lines after the diagnostics split.
- It mixes route rendering, settings store binding, unsaved-navigation blocking,
  cleanup preview/confirmation, reset/save orchestration, AI model/provider
  refresh, secret creation/deletion rollback, notification refresh, and tab
  composition.
- Route files should stay thin. This screen now owns multiple feature-level
  workflows that belong in focused hooks/controllers and panels under
  `apps/desktop/src/features/settings/`.

Tasks:

- [x] Inspect current settings screen tests, store contracts, settings feature
      helpers, and AI client usage before editing.
- [x] Move AI model/provider refresh, model actions, and provider-secret
      draft/save/delete orchestration into a focused settings AI controller
      hook.
- [x] Move cleanup preview, reset/save confirmation, and pending secret cleanup
      coordination into a focused save/reset workflow hook.
- [x] Move unsaved-navigation blocker wiring into a route-level helper hook or
      controller object.
- [x] Evaluate header actions and cleanup confirmation dialog ownership; keep
      them in the route once controller extraction makes them simple
      composition.
- [x] Keep `SettingsScreen.tsx` responsible only for route-level loading/error
      composition and passing controller props to tab components.
- [x] Preserve visible settings behavior, Tauri IPC calls, secret rollback
      behavior, notification refresh behavior, and leave-dialog behavior.
- [x] Verify with `COREPACK_HOME=/tmp/corepack corepack yarn lint`.
- [x] Verify with `COREPACK_HOME=/tmp/corepack corepack yarn typecheck`.
- [x] Verify with `COREPACK_HOME=/tmp/corepack corepack yarn test`.

Verification notes:

- Completed on 2026-06-12:
  - Moved AI model/provider refresh, model actions, and provider-secret
    save/delete rollback into `use-settings-ai-controller.ts`.
  - Moved settings store binding, cleanup preview/confirmation, save/reset
    orchestration, leave blocking, and route workflow state into
    `use-settings-screen-controller.ts`.
  - Kept `SettingsScreen.tsx` as route-level composition for header actions,
    dialogs, alerts, loading/error states, and tab wiring.
  - Current line counts: `SettingsScreen.tsx` 262,
    `use-settings-screen-controller.ts` 264, and
    `use-settings-ai-controller.ts` 250.
  - `COREPACK_HOME=/tmp/corepack corepack yarn lint:fix` passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn lint` passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn typecheck` passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn workspace @daydream/desktop test SettingsScreen.browser.test.tsx`
    passed with loopback binding permission: 13 tests.
  - `COREPACK_HOME=/tmp/corepack corepack yarn test` passed with loopback
    binding permission: desktop 263 tests, shared 9 tests, ffmpeg 1 test.

### 22. Split Linux portal ownership

Source: `apps/desktop/src-tauri/src/recorder/platform/linux/portal.rs`

Current problems:

- `portal.rs` is now the largest production Rust file at 622 lines after the
  settings screen split.
- It mixes portal health probing, source selection, live session lifecycle,
  restore-token lookup/storage/invalidation, stream-to-source mapping,
  cursor/source label helpers, portal error mapping, and tests.
- The file also owns several `#[allow(dead_code)]` suppressions for live portal
  session types and stream mapping helpers. Those should become narrow module
  boundaries or documented live-session exports, not broad clutter in one file.

Tasks:

- [x] Inspect Linux portal callers, live capture session usage, tests, and
      platform module exports before editing.
- [x] Create `apps/desktop/src-tauri/src/recorder/platform/linux/portal/mod.rs`
      as the portal entry.
- [x] Move health probing and source/cursor capability labels into
      `portal/health.rs`.
- [x] Move source chooser and restore-token reuse for selection into
      `portal/selection.rs`.
- [x] Move live portal session types, live restore-token validation, pipewire
      remote opening, and session close behavior into `portal/live_session.rs`.
- [x] Move restore-token constants and storage helpers into
      `portal/restore_token.rs`.
- [x] Move stream/source mapping helpers into `portal/streams.rs`.
- [x] Move portal error mapping into `portal/errors.rs`.
- [x] Move tests into `portal/tests.rs` or focused module-local tests.
- [x] Remove or narrow `#[allow(dead_code)]` suppressions where the split makes
      ownership explicit.
- [x] Preserve selection behavior, restore-token persistence semantics,
      permission-repairable errors, live session close behavior, and tests.
- [x] Verify with `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- [x] Verify with `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- [x] Verify with `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml recorder::platform::linux::portal::tests`.
- [x] Verify with `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`.
- [x] Verify with `COREPACK_HOME=/tmp/corepack corepack yarn lint`.

Verification notes:

- Completed on 2026-06-12:
  - Replaced flat Linux `portal.rs` with `portal/`.
  - Added `health.rs` for ScreenCast portal health and source/cursor
    capability labels.
  - Added `selection.rs` for chooser source selection, restore-token reuse,
    and selected-source persistence.
  - Added `live_session.rs` for live portal session types, live restore-token
    validation, PipeWire remote opening, and session close behavior.
  - Added `restore_token.rs` for restore-token IDs, storage invalidation, and
    permission-repairable restore-token errors.
  - Added `streams.rs` for portal stream to capture/live-source mapping.
  - Added `errors.rs` for portal error classification.
  - Added `tests.rs` for the existing portal helper tests.
  - Removed portal `#[allow(dead_code)]` suppressions; live session types and
    helpers are now explicit module exports.
  - Current line counts: `live_session.rs` 162, `selection.rs` 156,
    `tests.rs` 96, `health.rs` 80, `streams.rs` 79, `restore_token.rs` 58,
    `errors.rs` 22, and `mod.rs` 16.
  - `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml recorder::platform::linux::portal::tests`
    passed: 8 tests.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings.
  - `COREPACK_HOME=/tmp/corepack corepack yarn lint` passed.
  - Full `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` failed
    under the default sandbox only because 9 transcription tests could not bind
    loopback ports; the same command passed with loopback permission: 276
    tests.

### 23. Split recorder compositor ownership

Source: `apps/desktop/src-tauri/src/recorder/compositor.rs`

Current problems:

- `compositor.rs` is now the largest production Rust file at 609 lines after
  the Linux portal split.
- It mixes scene-to-runtime planning, even canvas dimension normalization,
  full-canvas passthrough detection, CPU canvas compositing, alpha blending,
  bilinear sampling, packed/YUV pixel conversion, and tests.
- The compositor is a correctness-sensitive capture boundary. The split must
  preserve frame byte privacy, source ordering, crop/rect math, writer handoff
  validation, and format conversion behavior.

Tasks:

- [x] Inspect compositor callers, capture runtime dependencies, media-writer
      handoff expectations, and tests before editing.
- [x] Create `apps/desktop/src-tauri/src/recorder/compositor/mod.rs` as the
      compositor entry.
- [x] Move scene planning structs and `from_scene` behavior into
      `compositor/plan.rs`.
- [x] Move full-canvas passthrough detection and composed-frame validation into
      `compositor/passthrough.rs` or keep as narrow plan methods if clearer.
- [x] Move CPU composition loops, crop/rect coordinate mapping, and alpha
      blending into `compositor/cpu.rs`.
- [x] Move bilinear sampling and packed/YUV pixel conversion into
      `compositor/pixels.rs`.
- [x] Move tests into `compositor/tests.rs` or focused module-local tests.
- [x] Preserve source z-ordering, odd-dimension padding, passthrough fast path,
      crop behavior, supported format handling, and test expectations.
- [x] Verify with `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- [x] Verify with `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- [x] Verify with `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml recorder::compositor::tests`.
- [x] Verify with `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`.
- [x] Verify with `COREPACK_HOME=/tmp/corepack corepack yarn lint`.

Verification notes:

- Completed on 2026-06-12:
  - Replaced flat `recorder/compositor.rs` with `recorder/compositor/`.
  - Added `plan.rs` for scene planning, even-dimension normalization,
    passthrough detection, and composed-frame validation.
  - Added `cpu.rs` for CPU composition loops, crop/rect coordinate mapping,
    and alpha blending.
  - Added `pixels.rs` for bilinear sampling and packed/YUV pixel conversion.
  - Added `tests.rs` for existing compositor behavior tests.
  - Current line counts: `tests.rs` 248, `plan.rs` 166, `cpu.rs` 114,
    `pixels.rs` 98, and `mod.rs` 9.
  - `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml recorder::compositor::tests`
    passed: 7 tests.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings.
  - `COREPACK_HOME=/tmp/corepack corepack yarn lint` passed.
  - Full `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` failed
    under the default sandbox only because 9 transcription tests could not bind
    loopback ports; the same command passed with loopback permission: 276
    tests.

### 24. Split library studio playback controller ownership

Source:
`apps/desktop/src/features/library/components/studio/use-recording-playback-controller.ts`

Current problems:

- `use-recording-playback-controller.ts` is now the largest production source
  file at 601 lines after the compositor split.
- It mixes media element refs, playback intent and segment advancement,
  timeline seeking/scrubbing, pause-gap resume timers, redaction state
  tracking, redaction-boundary scheduling, audio/video synchronization,
  mute/volume application, fullscreen refs, playback error handling, and the
  controller return contract.
- This is a user-visible playback workflow. The split must preserve timeline
  source provenance, redaction skip/mute behavior, audio/video sync, pause gap
  behavior, and browser test coverage.

Tasks:

- [x] Inspect library studio callers, playback browser tests, timeline store
      interactions, redaction helpers, and component prop usage before editing.
- [x] Move media element refs, registration, media settings application, and
      audio synchronization into a focused media-elements hook/helper.
- [x] Move redaction state calculation, redaction-boundary timers, and skip to
      next unredacted video into a focused redaction playback hook/helper.
- [x] Move segment advancement, pause-gap resume timers, and seek/scrub
      transitions into a focused timeline playback hook/helper.
- [x] Keep `useRecordingPlaybackController` as the orchestration boundary and
      stable return contract for existing studio components.
- [x] Preserve visible playback behavior, redaction behavior, fullscreen
      behavior, playback errors, and tests.
- [x] Verify with `COREPACK_HOME=/tmp/corepack corepack yarn lint`.
- [x] Verify with `COREPACK_HOME=/tmp/corepack corepack yarn typecheck`.
- [x] Verify with focused library playback/studio tests or document the closest
      available coverage.
- [x] Verify with `COREPACK_HOME=/tmp/corepack corepack yarn test`.

Verification notes:

- Completed on 2026-06-12:
  - Added `recording-playback-media-elements.ts` for media refs, audio
    registration, mute/volume application, audio sync, pause, and fullscreen
    element refs.
  - Added `recording-playback-redaction-controller.ts` for redaction state,
    redaction-boundary scheduling, video redaction activity, and skip-target
    calculation.
  - Added `recording-playback-resume-timer.ts` for pause-gap resume progress
    and timer cleanup.
  - Kept `useRecordingPlaybackController` as the orchestration boundary and
    stable return contract for studio components.
  - Current line counts: `use-recording-playback-controller.ts` 386,
    `recording-playback-redaction-controller.ts` 197,
    `recording-playback-media-elements.ts` 135, and
    `recording-playback-resume-timer.ts` 45.
  - `COREPACK_HOME=/tmp/corepack corepack yarn workspace @daydream/desktop lint`
    passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn workspace @daydream/desktop typecheck`
    passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn workspace @daydream/desktop test RecordingStudioScreen.browser.test.tsx`
    passed with loopback binding permission: 53 tests.
  - `COREPACK_HOME=/tmp/corepack corepack yarn lint` passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn typecheck` passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn test` passed with loopback
    binding permission: desktop 263 tests, shared 9 tests, ffmpeg 1 test.

### 25. Split library studio sidebar ownership

Source:
`apps/desktop/src/features/library/components/studio/studio-sidebar.tsx`

Current problems:

- `studio-sidebar.tsx` is now the largest production source file at 596 lines
  after the playback controller split.
- It mixes inspector selection routing, lane editing, clip editing, recording
  metadata, selected-range redaction actions, transcription summaries,
  transcription chunk aggregation, color palette controls, and type labels.
- This is part of the user-facing recording studio workflow. The split must
  preserve selection-specific inspector behavior, redaction actions, editable
  lane/clip fields, color selection, and transcription summaries.

Tasks:

- [x] Inspect sidebar callers, store actions, tests, and shared section
      components before editing.
- [x] Move transcription summary components and chunk aggregation helpers into
      a focused sidebar transcription module.
- [x] Move editable lane fields, color palette popover, and track kind labels
      into a focused sidebar lane editing module.
- [x] Move lane, segment, recording, and selected-range inspector sections into
      focused sidebar section modules or a section composition module.
- [x] Keep `StudioSidebar` responsible for reading selection state and choosing
      the correct inspector content.
- [x] Preserve visible inspector behavior, redaction actions, editable fields,
      and browser test expectations.
- [x] Verify with `COREPACK_HOME=/tmp/corepack corepack yarn lint`.
- [x] Verify with `COREPACK_HOME=/tmp/corepack corepack yarn typecheck`.
- [x] Verify with focused recording studio browser tests.
- [x] Verify with `COREPACK_HOME=/tmp/corepack corepack yarn test`.

Verification notes:

- Completed on 2026-06-12:
  - Added `studio-sidebar-transcription.tsx` for transcription track details,
    chunk details, selection summaries, chunk aggregation, and status labels.
  - Added `studio-sidebar-editing.tsx` for editable lane fields, the lane color
    palette popover, and track kind labels.
  - Added `studio-sidebar-inspector-sections.tsx` for lane, segment, selected
    range, and recording inspector section composition.
  - Kept `StudioSidebar` responsible for reading selection state, finding the
    selected model, and routing to the correct inspector section.
  - Current line counts: `studio-sidebar-inspector-sections.tsx` 217,
    `studio-sidebar.tsx` 191, `studio-sidebar-transcription.tsx` 119, and
    `studio-sidebar-editing.tsx` 107.
  - `COREPACK_HOME=/tmp/corepack corepack yarn workspace @daydream/desktop lint`
    passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn workspace @daydream/desktop typecheck`
    passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn workspace @daydream/desktop test RecordingStudioScreen.browser.test.tsx`
    passed with loopback binding permission: 53 tests.
  - `COREPACK_HOME=/tmp/corepack corepack yarn lint` passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn typecheck` passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn test` passed with loopback
    binding permission: desktop 263 tests, shared 9 tests, ffmpeg 1 test.

### 26. Split library studio track lane ownership

Source:
`apps/desktop/src/features/library/components/studio/track-lane.tsx`

Current problems:

- `track-lane.tsx` is now the largest production UI source file at 580 lines
  after the sidebar split.
- It mixes sticky lane header controls, lane pointer/click selection state,
  track clip layout, derived AI transcription layer placement, selected time
  range overlays, redaction overlays, waveform tile clipping/rendering, silence
  placeholders, and video thumbnail strips.
- This is part of the user-facing recording studio timeline. The split must
  preserve clip selection, lane selection, redaction selection, mute controls,
  waveform rendering, video thumbnail state text, and derived transcription
  overlays.

Tasks:

- [x] Inspect `TrackLane` callers, sortable lane wrapper, studio browser tests,
      waveform data contracts, thumbnail state, and derived AI lane usage before
      editing.
- [x] Move lane header controls into a focused lane-header component.
- [x] Move clip rendering and clip body media choice into a focused track-clip
      module.
- [x] Move selected range and redaction overlay rendering into a focused
      overlay module.
- [x] Move waveform tile, silence baseline, waveform shape, and video thumbnail
      strip rendering into focused media layer modules.
- [x] Keep `TrackLane` responsible for lane-level composition, selection
      gesture handling, and the stable `TrackLaneProps` contract used by
      `SortableTrackLane`.
- [x] Preserve visible studio timeline behavior and browser test expectations.
- [x] Verify with `COREPACK_HOME=/tmp/corepack corepack yarn lint`.
- [x] Verify with `COREPACK_HOME=/tmp/corepack corepack yarn typecheck`.
- [x] Verify with focused recording studio browser tests.
- [x] Verify with `COREPACK_HOME=/tmp/corepack corepack yarn test`.

Verification notes:

- Completed on 2026-06-12:
  - Added `track-lane-header.tsx` for the sticky lane label, reorder handle,
    mute control, and lane select button.
  - Added `track-clip.tsx` for clip shell layout, selection behavior, clip
    title strip, and derived transcription layer placement.
  - Added `track-lane-overlays.tsx` for selected time-range and redaction
    overlays.
  - Added `track-lane-media.tsx` for waveform tile rendering, silence
    baseline, waveform shape, and video thumbnail strips.
  - Kept `track-lane.tsx` responsible for lane composition, lane pointer
    movement detection, lane selection, and the stable `TrackLaneProps`
    contract used by `SortableTrackLane`.
  - Current line counts: `track-lane.tsx` 223, `track-lane-media.tsx` 161,
    `track-lane-header.tsx` 98, `track-clip.tsx` 93, and
    `track-lane-overlays.tsx` 82.
  - `COREPACK_HOME=/tmp/corepack corepack yarn workspace @daydream/desktop lint`
    passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn workspace @daydream/desktop typecheck`
    passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn workspace @daydream/desktop test RecordingStudioScreen.browser.test.tsx`
    passed with loopback binding permission: 53 tests.
  - `COREPACK_HOME=/tmp/corepack corepack yarn lint` passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn typecheck` passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn test` passed with loopback
    binding permission: desktop 263 tests, shared 9 tests, ffmpeg 1 test.

### 27. Split recording studio route ownership

Source:
`apps/desktop/src/screens/RecordingStudioScreen.tsx`

Current problems:

- `RecordingStudioScreen.tsx` is now the largest production UI source file at
  577 lines after the track lane split.
- It mixes route params and navigation, recording/detail loading, playback
  session lifecycle, thumbnail status, redaction mutation wrappers, AI
  processing start/reload state, delete and rename dialogs, debug JSON loading
  and clipboard behavior, route header rendering, empty/loading/error states,
  and recording capability predicates.
- Route files should stay thin. This screen should own route composition and
  top-level states while feature-specific loading, dialogs, toolbar actions,
  and debug behavior live under `apps/desktop/src/features/library/`.

Tasks:

- [x] Inspect the recording studio route tests, library client calls, playback
      session cleanup, debug toggle behavior, and dialog interactions before
      editing.
- [x] Move detail loading, playback session lifecycle, thumbnail status, and
      redaction mutation wrappers into a focused recording studio controller
      hook.
- [x] Move rename and delete dialogs into focused feature components.
- [x] Move debug JSON loading, copy status, and debug dialog rendering into a
      focused debug component or hook.
- [x] Move route header metadata and action buttons into a focused toolbar
      component.
- [x] Keep `RecordingStudioScreen` responsible for route params, navigation,
      top-level layout, and composing the recording studio feature.
- [x] Preserve visible loading, error, rename, delete, debug, refresh, AI
      processing, redaction, and playback behavior.
- [x] Verify with `COREPACK_HOME=/tmp/corepack corepack yarn lint`.
- [x] Verify with `COREPACK_HOME=/tmp/corepack corepack yarn typecheck`.
- [x] Verify with focused recording studio browser tests.
- [x] Verify with `COREPACK_HOME=/tmp/corepack corepack yarn test`.

Verification notes:

- Completed on 2026-06-12:
  - Added `use-recording-studio-controller.ts` for recording detail loading,
    playback session lifecycle, thumbnail status, redaction mutation wrappers,
    delete behavior, rename behavior, and AI processing start/reload behavior.
  - Added `recording-studio-toolbar.tsx` for route header metadata, refresh,
    debug, rename, delete, and AI processing actions.
  - Added `recording-rename-dialog.tsx` and `recording-delete-dialog.tsx` for
    route dialogs.
  - Added `recording-debug-dialog.tsx` for debug JSON loading, local clipboard
    copy status, and debug error rendering.
  - Added `recording-studio-route-utils.ts` for title and capability helpers.
  - Kept `RecordingStudioScreen.tsx` responsible for route params, navigation,
    top-level layout, dialog-open state, and feature composition.
  - Current line counts: `use-recording-studio-controller.ts` 249,
    `recording-studio-toolbar.tsx` 142, `recording-debug-dialog.tsx` 104,
    `RecordingStudioScreen.tsx` 95, `recording-rename-dialog.tsx` 91,
    `recording-delete-dialog.tsx` 41, and
    `recording-studio-route-utils.ts` 26.
  - `COREPACK_HOME=/tmp/corepack corepack yarn workspace @daydream/desktop lint`
    passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn workspace @daydream/desktop typecheck`
    passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn workspace @daydream/desktop test RecordingStudioScreen.browser.test.tsx`
    passed with loopback binding permission: 53 tests.
  - `COREPACK_HOME=/tmp/corepack corepack yarn lint` passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn typecheck` passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn test` passed with loopback
    binding permission: desktop 263 tests, shared 9 tests, ffmpeg 1 test.

### 28. Split shared AI contract ownership

Source:
`packages/shared/src/ai.ts`

Current problems:

- `ai.ts` is now the largest production TypeScript source file at 571 lines
  after the recording studio route split.
- It mixes settings contracts, local/remote provider status, model cache and
  model download inputs, processing jobs and chunks, transcript state,
  notification contracts, activity intervals, OCR regions, VLM observations,
  visual observation state, and AI result query inputs.
- This is a shared external boundary used by React and the Rust IPC layer. The
  split must preserve the public `@daydream/shared` exports and the existing
  `./ai.js` import path while separating contracts by ownership.

Tasks:

- [x] Inspect shared exports, package tests, TS consumers, and Rust type
      generation assumptions before editing.
- [x] Move AI settings, provider kinds, OCR settings, activity settings, and
      limits into a focused settings contract module.
- [x] Move model asset, backend status, provider status, model action input,
      and secret input/result contracts into a focused model/provider module.
- [x] Move processing job, chunk, job action, and recording processing state
      contracts into a focused processing contract module.
- [x] Move transcript and transcription track/chunk state contracts into a
      focused transcript contract module.
- [x] Move notification contracts into a focused notification contract module.
- [x] Move activity, OCR, VLM, visual region, and visual observation contracts
      into focused activity/visual result modules.
- [x] Keep `packages/shared/src/ai.ts` as the compatibility barrel for current
      imports.
- [x] Verify with `COREPACK_HOME=/tmp/corepack corepack yarn lint`.
- [x] Verify with `COREPACK_HOME=/tmp/corepack corepack yarn typecheck`.
- [x] Verify with `COREPACK_HOME=/tmp/corepack corepack yarn test`.

Verification notes:

- Completed on 2026-06-12:
  - Added shared AI contract modules under `packages/shared/src/ai/`:
    `provider-kinds.ts`, `settings.ts`, `models.ts`, `processing.ts`,
    `transcript.ts`, `notifications.ts`, `activity.ts`, `visual.ts`, and
    `results.ts`.
  - Kept `packages/shared/src/ai.ts` as the compatibility barrel for existing
    `@daydream/shared` and `./ai.js` imports.
  - Current line counts: `transcript.ts` 110, `settings.ts` 108,
    `models.ts` 103, `processing.ts` 94, `visual.ts` 68,
    `notifications.ts` 67, `activity.ts` 20, `provider-kinds.ts` 17,
    `ai.ts` 9, and `results.ts` 3.
  - `COREPACK_HOME=/tmp/corepack corepack yarn workspace @daydream/shared lint`
    passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn workspace @daydream/shared typecheck`
    passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn lint` passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn typecheck` passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn test` passed with loopback
    binding permission: desktop 263 tests, shared 9 tests, ffmpeg 1 test.

### 29. Split recorder workspace store ownership

Source:
`apps/desktop/src/features/recorder/recorder-workspace-store.ts`

Current problems:

- `recorder-workspace-store.ts` is now the largest production TypeScript file
  at 570 lines after the shared AI contract split.
- It mixes Zustand state shape, workspace load/save, scene source deletion,
  duplicate capture-source resolution, audio-source selection, draft scene
  updates, preview start/stop lifecycle, preview frame subscriptions, recorder
  status subscriptions, recording lifecycle actions, recording stats refresh,
  capture source selection, portal-cancel handling, and client command wiring.
- This store drives the capture workflow. The split must preserve visible scene
  editing, preview behavior, recorder lifecycle locking, duplicate-source
  prompts, audio source persistence, and existing browser/store tests.

Tasks:

- [x] Inspect recorder workspace tests, Capture screen interactions, helper
      modules, preview store helpers, lifecycle helpers, and client calls before
      editing.
- [x] Move state type definitions and initial state construction into a focused
      store types/state module.
- [x] Move load/save/capture-source update actions into a focused workspace
      persistence/source-selection action module.
- [x] Move scene edit actions, audio selection, delete source, and duplicate
      source resolution into a focused scene-edit action module.
- [x] Move preview start/stop/frame subscription actions into a focused preview
      action module.
- [x] Move recording lifecycle, status subscription, and recording stats actions
      into a focused lifecycle/stats action module.
- [x] Keep `recorder-workspace-store.ts` responsible for composing the Zustand
      store and preserving the `useRecorderWorkspaceStore` public contract.
- [x] Preserve Capture screen behavior and store tests.
- [x] Verify with `COREPACK_HOME=/tmp/corepack corepack yarn lint`.
- [x] Verify with `COREPACK_HOME=/tmp/corepack corepack yarn typecheck`.
- [x] Verify with focused recorder workspace store and Capture screen tests.
- [x] Verify with `COREPACK_HOME=/tmp/corepack corepack yarn test`.

Verification notes:

- Completed on 2026-06-12:
  - Added `recorder-workspace-store-types.ts` for the public Zustand state,
    patch, set/get, async status, and lifecycle action types.
  - Added `recorder-workspace-initial-state.ts` for initial state values.
  - Added `recorder-workspace-persistence-actions.ts` for load, save scene,
    capture source selection, active-preview shutdown before source selection,
    and portal cancellation handling.
  - Added `recorder-workspace-scene-actions.ts` for scene source deletion,
    duplicate capture-source resolution, audio source selection, direct scene
    selection, and draft scene updates.
  - Added `recorder-workspace-preview-actions.ts` for preview start/stop and
    preview frame subscriptions.
  - Added `recorder-workspace-lifecycle-actions.ts` for start/pause/resume/stop,
    recorder status subscriptions, and recording stats refresh.
  - Kept `recorder-workspace-store.ts` as the 25-line Zustand composer and
    compatibility export surface for existing type imports.
  - Current line counts: `recorder-workspace-persistence-actions.ts` 176,
    `recorder-workspace-scene-actions.ts` 175,
    `recorder-workspace-lifecycle-actions.ts` 165,
    `recorder-workspace-preview-actions.ts` 127,
    `recorder-workspace-store-types.ts` 63,
    `recorder-workspace-initial-state.ts` 42, and
    `recorder-workspace-store.ts` 25.
  - `COREPACK_HOME=/tmp/corepack corepack yarn workspace @daydream/desktop lint`
    passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn workspace @daydream/desktop typecheck`
    passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn workspace @daydream/desktop test recorder-workspace-store.test.ts`
    passed: 40 tests.
  - `COREPACK_HOME=/tmp/corepack corepack yarn workspace @daydream/desktop test CaptureScreen.browser.test.tsx`
    passed with loopback binding permission: 19 tests.
  - `COREPACK_HOME=/tmp/corepack corepack yarn lint` passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn typecheck` passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn test` passed with loopback
    binding permission: desktop 263 tests, shared 9 tests, ffmpeg 1 test.

### 30. Split library studio track timeline ownership

Source:
`apps/desktop/src/features/library/components/studio/track-timeline.tsx`

Current problems:

- `track-timeline.tsx` is now the largest production TypeScript file at 563
  lines after the recorder workspace store split.
- It mixes timeline viewport measurement, scroll state, playhead-follow
  behavior, programmatic scroll suppression, wheel zoom/scroll handling, visible
  window zoom math, waveform request window derivation, lane ordering and drag
  sensors, selection wiring, redaction/unredaction actions, toolbar state, range
  labels, ruler rendering, playhead rendering, and lane rendering.
- This is a performance-sensitive UI path. Repeated range filtering per lane,
  scroll effects, wheel handlers, and dense child props must stay predictable
  while the split preserves timeline selection, zoom, shift-wheel scroll,
  playhead follow, redaction actions, waveform loading, and drag reorder tests.

Tasks:

- [x] Inspect recording studio browser tests, timeline selection hook,
      track-timeline parts, waveform queue hook, and playback controller
      interactions before editing.
- [x] Move viewport width, scroll state, programmatic scroll suppression,
      follow-playhead behavior, and playhead visibility into a focused viewport
      hook.
- [x] Move zoom and wheel handling into a focused timeline zoom/scroll hook.
- [x] Move derived timeline view model data, including selected-id sets,
      ordered tracks, ruler ticks, waveform request range, and per-lane range
      grouping, into focused helpers or a view-model hook.
- [x] Move redaction/unredaction action construction into a focused selection
      action helper.
- [x] Keep `TrackTimeline` responsible for feature composition and the stable
      props consumed by `RecordingStudio`.
- [x] Preserve visible recording studio timeline behavior and browser tests.
- [x] Verify with `COREPACK_HOME=/tmp/corepack corepack yarn lint`.
- [x] Verify with `COREPACK_HOME=/tmp/corepack corepack yarn typecheck`.
- [x] Verify with focused recording studio browser tests.
- [x] Verify with `COREPACK_HOME=/tmp/corepack corepack yarn test`.

Verification notes:

- Completed on 2026-06-12:
  - Added `use-track-timeline-viewport.ts` for viewport refs, width
    measurement, scroll state, programmatic scroll suppression, playhead-follow
    behavior, and seek-revision follow reset.
  - Added `use-track-timeline-zoom-scroll.ts` for ctrl-wheel zoom,
    shift-wheel horizontal scrolling, zoom anchoring, and visible window
    updates.
  - Added `track-timeline-view-model.ts` for ordered tracks, selected-id sets,
    ruler ticks, waveform request range, playhead position, zoom status, and
    pre-grouped per-lane selected ranges.
  - Added `track-timeline-redaction-actions.ts` for mapping selected ranges to
    redaction command inputs.
  - Kept `TrackTimeline` responsible for feature composition, DnD lane
    ordering, store wiring, toolbar callbacks, and lane rendering.
  - Current line counts: `track-timeline.tsx` 299,
    `use-track-timeline-viewport.ts` 178, `track-timeline-view-model.ts` 159,
    `use-track-timeline-zoom-scroll.ts` 152, and
    `track-timeline-redaction-actions.ts` 12.
  - `COREPACK_HOME=/tmp/corepack corepack yarn workspace @daydream/desktop lint`
    passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn workspace @daydream/desktop typecheck`
    passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn workspace @daydream/desktop test RecordingStudioScreen.browser.test.tsx`
    passed with loopback binding permission: 53 tests.
  - `COREPACK_HOME=/tmp/corepack corepack yarn lint` passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn typecheck` passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn test` passed with loopback
    binding permission: desktop 263 tests, shared 9 tests, ffmpeg 1 test.

### 31. Split AI Tauri command ownership

Source:
`apps/desktop/src-tauri/src/ai/commands.rs`

Current problems:

- `ai/commands.rs` was the largest production source file at 542 lines after
  the track timeline split.
- It mixes Tauri model cache commands, download request state handling, local
  model self-test orchestration, provider status construction, provider secret
  validation and persistence, processing job action commands, recording result
  query commands, and background worker startup.
- Command handlers should stay thin. The split must preserve IPC command names,
  notification behavior, model list events, processing-state events, local
  secret validation, worker startup, and existing Rust/TypeScript tests.

Tasks:

- [x] Inspect `ai/mod.rs`, command registrations, AI tests, model helpers,
      processing helpers, result helpers, and notification helpers before
      editing.
- [x] Move model list/download/cancel/delete/self-test command logic into a
      focused command module.
- [x] Move provider status and provider secret command logic into a focused
      provider command module.
- [x] Move processing job action commands and recording result query commands
      into focused processing/result command modules.
- [x] Move worker startup orchestration into a focused worker command/startup
      module.
- [x] Keep `ai/commands.rs` as the compatibility command export surface or
      command module aggregator used by `lib.rs`.
- [x] Preserve Tauri command names and event behavior.
- [x] Verify with `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- [x] Verify with `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- [x] Verify with focused AI Rust tests or document the closest available
      coverage.
- [x] Verify with `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`.
- [x] Verify with `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml`.

Verification notes:

- Completed on 2026-06-12:
  - Replaced `ai/commands.rs` with a 5-module command aggregator.
  - Added `ai/commands/model_commands.rs` for model list, download, cancel,
    delete, and local model self-test commands.
  - Added `ai/commands/provider_commands.rs` for provider status and provider
    secret validation/persistence commands.
  - Added `ai/commands/processing_commands.rs` for processing job listing,
    pause/resume/cancel/retry actions, and recording processing enqueue.
  - Added `ai/commands/result_commands.rs` for recording processing state,
    transcript, activity, and visual observation read commands.
  - Added `ai/commands/worker_startup.rs` for background processing worker
    startup and stale processing recovery.
  - Updated `lib.rs` to register the focused command module paths because the
    Tauri macro helper symbols are generated in the module where each command
    function is defined; IPC command names remain preserved by the existing
    `#[tauri::command(rename = ...)]` attributes.
  - Current command line counts: `commands.rs` 5,
    `commands/model_commands.rs` 292,
    `commands/provider_commands.rs` 132,
    `commands/processing_commands.rs` 73,
    `commands/result_commands.rs` 33, and
    `commands/worker_startup.rs` 16.
  - `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings.
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml ai::tests`
    passed: 44 tests.
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` failed under
    the default sandbox only because transcription tests could not bind
    loopback ports, then passed with loopback permission: 276 tests.

### 32. Split recorder command ownership

Source:
`apps/desktop/src-tauri/src/recorder/commands.rs`

Current problems:

- `recorder/commands.rs` is 537 lines and mixes status/stat read commands,
  capture source selection, scene persistence, preview lifecycle, live recording
  start/resume/pause/stop orchestration, input activity monitor attachment, and
  AI auto-processing side effects after stop.
- The root `recorder/mod.rs` already owns the Tauri command macro wrappers, so
  this split can keep external command paths stable while moving implementation
  ownership into focused internal modules.
- The lifecycle code has meaningful state-transition risk. It must preserve
  stale preview generation handling, selecting-state rollback, segment row
  creation/failure behavior, recorder status events, and AI auto-start after
  stop.

Tasks:

- [x] Inspect `recorder/mod.rs`, manager lifecycle modules, platform helpers,
      storage mapping helpers, recorder tests, and command registration before
      editing.
- [x] Move status and recording stats command logic into a focused command
      module.
- [x] Move capture source selection and scene persistence command logic into a
      focused command module.
- [x] Move preview start/stop/status command logic into a focused preview
      command module.
- [x] Move live recording lifecycle commands and lifecycle completion side
      effects into a focused lifecycle command module.
- [x] Keep `recorder/commands.rs` as an internal aggregator used by
      `recorder/mod.rs`.
- [x] Preserve Tauri command names, recorder events, logging, and AI auto-start
      behavior.
- [x] Verify with `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- [x] Verify with `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- [x] Verify with focused recorder Rust tests or document the closest available
      coverage.
- [x] Verify with `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`.
- [x] Verify with `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml`.

Verification notes:

- Completed on 2026-06-12:
  - Replaced `recorder/commands.rs` with a 4-module internal command
    aggregator used by the existing Tauri wrappers in `recorder/mod.rs`.
  - Added `recorder/commands/status_commands.rs` for recorder status,
    performance stats, active recording stats, active segment selection, and
    segment file-size reads.
  - Added `recorder/commands/source_commands.rs` for capture source listing,
    source selection, audio source listing, scene reads, scene saves, and the
    configuration-lock guard.
  - Added `recorder/commands/preview_commands.rs` for preview status,
    start/stop, stale generation handling, empty-scene preview stop, and
    preview activation.
  - Added `recorder/commands/lifecycle_commands.rs` for recording
    start/pause/resume/stop, activity monitor attachment, lifecycle completion
    event emission, and AI auto-processing after stop.
  - Kept test-only helper re-exports behind `cfg(test)` so normal builds do not
    carry unused helper imports.
  - Current command line counts: `commands.rs` 25,
    `commands/lifecycle_commands.rs` 276,
    `commands/preview_commands.rs` 133,
    `commands/source_commands.rs` 66, and
    `commands/status_commands.rs` 90.
  - `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings.
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml recorder::tests`
    passed: 34 tests.
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` failed under
    the default sandbox only because transcription tests could not bind
    loopback ports, then passed with loopback permission: 276 tests.

### 33. Split library overview screen ownership

Source:
`apps/desktop/src/screens/LibraryScreen.tsx`

Current problems:

- `LibraryScreen.tsx` is 523 lines and mixes route composition, data loading,
  thumbnail queue cancellation, selection/range-selection state, aggregate
  size derivation, delete/rename mutations, navigation, toolbar rendering,
  grid/list rendering, and dialog rendering.
- The issue is not only size. Selection checks use repeated array membership
  during list rendering, delete target derivation repeatedly searches
  recordings, and async thumbnail loading state is owned directly by the route
  component instead of a focused controller.
- The route should stay thin and own feature composition. Overview-specific
  state transitions, derived view model data, dialogs, and toolbar/list
  sections should move under `apps/desktop/src/features/library/components/overview/`.

Tasks:

- [x] Inspect current library overview utilities, item component, browser tests,
      route usage, and library client functions before editing.
- [x] Move load/reload, thumbnail run cancellation, delete, rename, selection,
      sort, and view-mode state into a `useLibraryOverviewController` hook or
      similarly focused controller.
- [x] Derive selected id membership and recordings-by-id maps once per render
      instead of repeated `includes`/`find` work inside render callbacks.
- [x] Move toolbar/header actions into a focused overview toolbar component.
- [x] Move empty/loading/error list rendering into a focused overview content
      component.
- [x] Move delete and rename dialogs into focused overview dialog components.
- [x] Keep `LibraryScreen.tsx` responsible only for route composition and
      high-level layout.
- [x] Preserve user-visible behavior, shadcn component usage, accessible names,
      and existing IPC calls.
- [x] Verify with `COREPACK_HOME=/tmp/corepack corepack yarn workspace @daydream/desktop lint`.
- [x] Verify with `COREPACK_HOME=/tmp/corepack corepack yarn workspace @daydream/desktop typecheck`.
- [x] Verify with focused library/browser tests or document the closest
      available coverage.
- [x] Verify with full `COREPACK_HOME=/tmp/corepack corepack yarn lint`,
      `typecheck`, and `test` before finalizing the item.

Verification notes:

- Completed on 2026-06-12:
  - Replaced `LibraryScreen.tsx` with a 32-line route shell that only assembles
    page layout, header actions, overview content, and dialogs.
  - Added `use-library-overview-controller.ts` for loading/reloading,
    thumbnail run cancellation, delete/rename mutations, selection/range
    selection, sort/view-mode state, navigation, and error state.
  - The controller now derives `selectedIdSet` and `recordingsById` once per
    render, removing repeated `includes` and `find` work from rendered item
    callbacks and delete target derivation.
  - Added `library-overview-toolbar.tsx` for count/size summary, selected
    delete action, sort control, view-mode buttons, and refresh.
  - Added `library-overview-content.tsx` for error/loading/empty states and
    grid/list item rendering.
  - Added `library-overview-dialogs.tsx` for delete and rename dialogs.
  - Current overview line counts: `LibraryScreen.tsx` 32,
    `use-library-overview-controller.ts` 415,
    `library-overview-dialogs.tsx` 117,
    `library-overview-toolbar.tsx` 102, and
    `library-overview-content.tsx` 79.
  - `COREPACK_HOME=/tmp/corepack corepack yarn workspace @daydream/desktop lint`
    passed after `lint:fix` corrected formatting in the new files.
  - `COREPACK_HOME=/tmp/corepack corepack yarn workspace @daydream/desktop typecheck`
    passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn workspace @daydream/desktop test LibraryScreen.browser.test.tsx`
    failed under the default sandbox only because Vitest could not bind its
    loopback browser server, then passed with loopback permission: 6 tests.
  - `COREPACK_HOME=/tmp/corepack corepack yarn lint` passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn typecheck` passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn test` failed under the default
    sandbox only because Vitest could not bind its loopback browser server,
    then passed with loopback permission: desktop 263 tests, shared 9 tests,
    ffmpeg 1 test.

### 34. Split diagnostics screen ownership

Source:
`apps/desktop/src/screens/DiagnosticsScreen.tsx`

Current problems:

- `DiagnosticsScreen.tsx` is now the largest production route file at 534
  lines.
- It mixes debug-gate routing, diagnostics loading, raw JSON copy, debug bundle
  copy, copy status rendering, and every diagnostics section in one route
  component.
- Most of the file is verbose diagnostic presentation. That is still an
  ownership problem because adding a new diagnostics domain requires editing a
  large screen instead of a focused diagnostics section.

Tasks:

- [x] Inspect diagnostics primitives, diagnostics store/client, browser tests,
      and settings debug hook before editing.
- [x] Move diagnostics copy actions and copy status state into a focused
      controller hook.
- [x] Move application/system/storage/debug logging sections into focused
      diagnostics section components.
- [x] Move recorder backend health and database/portal sections into focused
      diagnostics section components.
- [x] Keep `DiagnosticsScreen.tsx` responsible only for debug gate, page header,
      error display, and section composition.
- [x] Preserve copy behavior, debug gating, accessible actions, and raw
      diagnostics JSON rendering.
- [x] Verify with `COREPACK_HOME=/tmp/corepack corepack yarn workspace @daydream/desktop lint`.
- [x] Verify with `COREPACK_HOME=/tmp/corepack corepack yarn workspace @daydream/desktop typecheck`.
- [x] Verify with focused diagnostics browser tests or document the closest
      available coverage.
- [x] Verify with full `COREPACK_HOME=/tmp/corepack corepack yarn lint`,
      `typecheck`, and `test` before finalizing the item.

Verification notes:

- Completed on 2026-06-12:
  - Replaced `DiagnosticsScreen.tsx` with a 74-line route shell that owns the
    debug-disabled gate, page header, error display, and diagnostics section
    composition.
  - Added `use-diagnostics-screen-controller.ts` for diagnostics loading,
    refresh, raw JSON copy, debug bundle copy, and copy status state.
  - Added `diagnostics-header-actions.tsx` for copy/refresh buttons and
    aria-live copy result text.
  - Added `diagnostics-summary-sections.tsx` for application, system, media
    writer, storage, and debug logging sections.
  - Added `recorder-backend-diagnostics-section.tsx`,
    `database-diagnostics-section.tsx`,
    `portal-restore-token-diagnostics-section.tsx`, and
    `raw-diagnostics-section.tsx` for the remaining diagnostics domains.
  - Current diagnostics line counts: `DiagnosticsScreen.tsx` 74,
    `use-diagnostics-screen-controller.ts` 86,
    `diagnostics-header-actions.tsx` 55,
    `diagnostics-summary-sections.tsx` 133,
    `recorder-backend-diagnostics-section.tsx` 173,
    `database-diagnostics-section.tsx` 55,
    `portal-restore-token-diagnostics-section.tsx` 66, and
    `raw-diagnostics-section.tsx` 21.
  - `COREPACK_HOME=/tmp/corepack corepack yarn workspace @daydream/desktop lint`
    passed after `lint:fix` formatted the new files.
  - `COREPACK_HOME=/tmp/corepack corepack yarn workspace @daydream/desktop typecheck`
    passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn workspace @daydream/desktop test DiagnosticsScreen.browser.test.tsx`
    failed under the default sandbox only because Vitest could not bind its
    loopback browser server, then passed with loopback permission: 4 tests.
  - `COREPACK_HOME=/tmp/corepack corepack yarn lint` passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn typecheck` passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn test` failed under the default
    sandbox only because Vitest could not bind its loopback browser server,
    then passed with loopback permission: desktop 263 tests, shared 9 tests,
    ffmpeg 1 test.

### 35. Split Rust notification ownership

Source:
`apps/desktop/src-tauri/src/notifications.rs`

Current problems:

- `notifications.rs` is now the largest production source file at 513 lines.
- It mixes serialized IPC DTOs, Tauri command handlers, retention cleanup,
  notification row mapping, action JSON parsing, action dispatch,
  processing-job mutation, unavailable fallback notifications, storage error
  mapping, time helpers, and tests.
- Notification actions are a cross-feature boundary. The command surface should
  stay thin, and action execution should be isolated enough that adding new
  action handlers does not require editing the DTO/command/mapping module.

Tasks:

- [x] Inspect notification client/store/tests, shared notification contracts,
      AI notification helpers, storage notification helpers, and `lib.rs`
      command registration before editing.
- [x] Move notification DTOs and `NotificationError` into a focused
      `notifications/types.rs` module.
- [x] Move row mapping, action JSON parsing, retention filtering, and storage
      error conversion into focused internal modules.
- [x] Move notification action execution into a focused action dispatcher
      module, with processing-job action helpers separated from model-download
      unavailable fallbacks.
- [x] Keep Tauri command functions registered under the existing
      `notifications::*` paths, using thin wrappers if needed so macro helper
      symbols remain available to `tauri::generate_handler!`.
- [x] Move notification tests beside the modules they cover or into
      `notifications/tests.rs`.
- [x] Preserve IPC command names, notification payload shapes, retention
      behavior, retry/pause/resume/cancel job semantics, and unavailable action
      notifications.
- [x] Verify with `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- [x] Verify with `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- [x] Verify with focused notification Rust tests or document the closest
      available coverage.
- [x] Verify with `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`.
- [x] Verify with `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml`.

Verification notes:

- Completed on 2026-06-12:
  - Replaced `notifications.rs` with thin Tauri command wrappers so existing
    `notifications::*` command registration paths and macro helper symbols stay
    intact.
  - Added `notifications/types.rs` for notification DTOs, command inputs, and
    `NotificationError`.
  - Added `notifications/commands.rs` for command implementation and retained
    notification list refresh behavior.
  - Added `notifications/mapping.rs` for row-to-DTO mapping, action JSON
    parsing, JSON-to-SQL error conversion, and storage error mapping.
  - Added `notifications/retention.rs` for retention cleanup and retained list
    reads.
  - Added `notifications/actions.rs` for stored action dispatch, model-download
    unavailable fallbacks, processing job update actions, and retry action
    requeue behavior.
  - Added `notifications/time.rs` and moved existing tests into
    `notifications/tests.rs`.
  - Current notification line counts: `notifications.rs` 65,
    `notifications/actions.rs` 184, `notifications/commands.rs` 95,
    `notifications/types.rs` 64, `notifications/mapping.rs` 42,
    `notifications/tests.rs` 121, `notifications/retention.rs` 11, and
    `notifications/time.rs` 8.
  - `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings.
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml notifications::tests`
    passed: 2 tests.
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` failed under
    the default sandbox only because transcription tests could not bind
    loopback ports, then passed with loopback permission: 276 tests.

### 36. Split studio timeline store ownership

Source:
`apps/desktop/src/features/library/components/studio/studio-timeline-store.ts`

Current problems:

- `studio-timeline-store.ts` is 488 lines and owns Zustand store creation,
  state types, lane/clip draft construction, selection normalization, range
  merging, range subtraction, pruning, color validation, ID toggling, lane
  ordering, and exported test helpers.
- The problem is state-transition complexity more than raw line count.
  Selection and range operations are central to timeline editing and redaction,
  so adding behavior here risks regressions across many workflows.
- Store files should compose focused action groups and deterministic reducers,
  not own every helper and reducer inline.

Tasks:

- [x] Inspect studio timeline tests, timeline selection hook, sidebar/timeline
      consumers, and existing studio timeline math before editing.
- [x] Move state/type definitions and `emptySelection` into a focused store
      types/initial-state module.
- [x] Move draft construction (`buildTimelineDrafts`, `studioClipId`) into a
      focused initialization module.
- [x] Move selection normalization, range merge/subtract, lane ID normalization,
      toggle, and lane reorder helpers into focused deterministic reducer
      modules.
- [x] Keep `studio-timeline-store.ts` as a small Zustand composition module with
      the existing public hook and exported helper surface preserved.
- [x] Preserve current selection behavior, redaction range behavior, lane
      ordering, color validation, and tests.
- [x] Verify with `COREPACK_HOME=/tmp/corepack corepack yarn workspace @daydream/desktop lint`.
- [x] Verify with `COREPACK_HOME=/tmp/corepack corepack yarn workspace @daydream/desktop typecheck`.
- [x] Verify with focused studio timeline tests or document the closest
      available coverage.
- [x] Verify with full `COREPACK_HOME=/tmp/corepack corepack yarn lint`,
      `typecheck`, and `test` before finalizing the item.

Verification notes:

- Replaced `studio-timeline-store.ts` with a 189-line Zustand composition module
  that preserves the existing import surface for the hook, colors, draft IDs,
  range helpers, and public types.
- Added `studio-timeline-store-types.ts` for store state, draft types,
  selection types, and `emptySelection`.
- Added `studio-timeline-drafts.ts` for `studioClipId` and
  `buildTimelineDrafts`; same-recording lane reconciliation now uses `Set`
  membership instead of repeated `includes` scans.
- Added `studio-timeline-selection.ts` for range IDs, range normalization,
  merge/subtract behavior, selection pruning, and ID toggling.
- Added `studio-timeline-lane-order.ts` for lane reordering.
- Line counts after split: `studio-timeline-store.ts` 189,
  `studio-timeline-store-types.ts` 63, `studio-timeline-drafts.ts` 59,
  `studio-timeline-selection.ts` 208, `studio-timeline-lane-order.ts` 16.
- `COREPACK_HOME=/tmp/corepack corepack yarn workspace @daydream/desktop lint`
  passed.
- `COREPACK_HOME=/tmp/corepack corepack yarn workspace @daydream/desktop typecheck`
  passed.
- `COREPACK_HOME=/tmp/corepack corepack yarn workspace @daydream/desktop test studio-timeline-store.test.ts`
  passed: 1 file, 15 tests.
- `COREPACK_HOME=/tmp/corepack corepack yarn workspace @daydream/desktop test RecordingStudioScreen.browser.test.tsx`
  failed under the default sandbox only because Vitest could not bind a
  loopback port, then passed with loopback permission: 1 file, 53 tests.
- `COREPACK_HOME=/tmp/corepack corepack yarn lint` passed.
- `COREPACK_HOME=/tmp/corepack corepack yarn typecheck` passed.
- `COREPACK_HOME=/tmp/corepack corepack yarn test` failed under the default
  sandbox only because Vitest could not bind a loopback port, then passed with
  loopback permission: desktop 28 files / 263 tests, shared 1 file / 9 tests,
  ffmpeg 1 file / 1 test.

### 37. Split AI audio processing ownership

Source:
`apps/desktop/src-tauri/src/ai/processing/audio.rs`

Current problems:

- `audio.rs` is 489 lines and owns chunk lookup, track/segment validation,
  offset math, FFmpeg extraction, temporary audio range handling, silence-gated
  speech range planning, provider transcription calls, transcript cleanup, and
  transcript persistence.
- The biggest risk is mixed side-effect ownership: filesystem/FFmpeg work,
  provider calls, storage mutation, and timeline provenance math all live in one
  flow. A future change to silence gating or transcript persistence can easily
  alter extraction semantics or redaction provenance by accident.
- The helper functions at the bottom are deterministic and already testable, but
  they are buried below the async orchestration. The orchestration should read
  as a sequence of explicit phases.

Tasks:

- [x] Inspect existing AI processing tests, transcription tests, storage
      transcript helpers, and worker execution call sites before editing.
- [x] Move chunk/track/segment loading and offset validation into an audio
      chunk input module with explicit structs for validated stream index,
      playback offsets, and slice duration.
- [x] Move FFmpeg extraction args and extraction execution into an audio
      extraction module that owns temp file path usage and cancellation checks.
- [x] Move silence gate range planning and per-range audio extraction into an
      audio range module.
- [x] Move transcript persistence DTOs/helpers into a transcript persistence
      module while preserving transaction behavior and timestamp provenance.
- [x] Keep `process_audio_chunk_inner` as a small orchestration function that
      preserves current logging semantics, result codes, cancellation behavior,
      provider selection, and storage cleanup.
- [x] Preserve current unit coverage for FFmpeg args, transcript word filtering,
      offsets, empty transcript persistence, silence-skip behavior, and
      successful transcription persistence.
- [x] Verify with `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- [x] Verify with `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- [x] Verify with `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`.
- [x] Verify with focused AI/audio processing tests or document the closest
      available coverage.
- [x] Verify with full `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml`
      before finalizing the item.

Verification notes:

- Replaced `audio.rs` with a 177-line orchestration module and private
  submodules for validated input, FFmpeg extraction, speech ranges, and
  transcript persistence.
- Added `audio/chunk_input.rs` for track/segment loading, stream index
  selection, chunk offset validation, and slice duration derivation.
- Added `audio/extraction.rs` for FFmpeg command args, temp output cleanup, and
  cancellation checks around extraction.
- Added `audio/ranges.rs` for silence-gate range planning and per-range temp
  audio extraction.
- Added `audio/transcript_persistence.rs` for pending transcription DTOs,
  transaction-scoped transcript segment/word persistence, word filtering, and
  timestamp offset math.
- Preserved existing `processing/mod.rs` test exports for `ffmpeg_audio_extract_args`,
  `persist_transcriptions_for_chunk`, `transcript_segment_offsets_ms`,
  `transcription_words_for_segment`, and `PendingTranscription`.
- Line counts after split: `audio.rs` 177, `audio/chunk_input.rs` 88,
  `audio/extraction.rs` 63, `audio/ranges.rs` 76,
  `audio/transcript_persistence.rs` 183.
- `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
- `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
- `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml ai::tests::`
  passed: 44 tests.
- `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
  passed without warnings.
- `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` failed under
  the default sandbox only because transcription tests could not bind loopback
  ports, then passed with loopback permission: 276 tests.

### 38. Split AI processing worker execution state machine

Source:
`apps/desktop/src-tauri/src/ai/processing/worker/execution.rs`

Current problems:

- `execution.rs` is 474 lines and owns the processing job loop, paused and
  cancelled job transitions, queued chunk claiming, source dispatch, per-source
  start/success/failure logging, cancellation reclassification, progress
  notifications, state emission, and chunk status persistence.
- The problem is branching/state-machine complexity. Audio, activity, and visual
  chunk execution repeat the same lifecycle shape with different inner
  processors and slightly different cancel error codes.
- Future source kinds or retry behavior should not require copying another large
  match arm with hand-maintained logging and status updates.

Tasks:

- [x] Inspect worker tests, scheduler, enqueue, completion, cancel monitor, and
      notification helpers before editing.
- [x] Move job-level status handling for paused/cancelled/running states into a
      focused job transition module.
- [x] Move queued chunk claiming and unsupported source handling into a dispatch
      module with explicit source-kind classification.
- [x] Move shared chunk lifecycle context/status helpers into a context module.
- [x] Reduce duplicated audio/activity/visual execution branches through a
      shared chunk runner or clearly separated per-source modules with common
      cancellation mapping.
- [x] Preserve current notifications, emitted processing-state events, error
      codes, chunk result-code handling, and logging fields.
- [x] Verify with focused AI worker/processing tests.
- [x] Verify with `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- [x] Verify with `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- [x] Verify with `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`.
- [x] Verify with full `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml`
      before finalizing the item.

Verification notes:

- Replaced `execution.rs` with a 46-line job loop that delegates job-state
  handling, chunk claiming/source classification, and chunk execution.
- Added `worker/job_state.rs` for paused/cancelled/running job-state decisions,
  chunk requeue/cancel transitions, processing notifications, and state
  emission.
- Added `worker/dispatch.rs` for queued chunk claiming, source-kind
  classification, unsupported-source errors, and claimed-chunk logging.
- Added `worker/chunk_context.rs` for shared chunk status persistence, result
  code handling, progress notification updates, current-job loading, and
  processing-state emission.
- Added `worker/chunk_runner.rs` for audio/activity/visual runners, shared
  cancel-monitor setup, and common pause/cancel reclassification.
- Line counts after split: `execution.rs` 46, `job_state.rs` 91,
  `dispatch.rs` 50, `chunk_context.rs` 88, `chunk_runner.rs` 291.
- `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
- `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
- `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml ai::tests::`
  passed: 44 tests.
- `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
  passed without warnings.
- `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` failed under
  the default sandbox only because transcription tests could not bind loopback
  ports, then passed with loopback permission: 276 tests.

## Additional Offender Discovery

Repeat this after each major cleanup pass.

- [x] Re-run size analysis excluding `target/`, `vendor/`, generated files, and
      `node_modules`.
- [x] Re-run `corepack yarn lint`.
- [x] Re-run `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`.
- [x] Search for broad suppressions: `allow(dead_code)`, `allow(clippy::*)`,
      `eslint-disable`, `@ts-ignore`, `@ts-expect-error`, and `as any`.
- [x] Search for large files and modules crossing ownership boundaries.
- [x] Add any new offender to this checklist before fixing it.

Current near misses to reassess:

- [x] `apps/desktop/src-tauri/src/recorder/capture_runtime.rs`: shared capture
      runtime split completed; no new runtime target added per the user stop
      instruction.
- [x] `apps/desktop/src-tauri/src/recorder/compositor.rs`: scene compositor
      split completed in item 23.
- [x] `apps/desktop/src-tauri/src/storage/tests.rs`: large storage integration
      test file. Reassessed as a residual test-organization risk, not a new
      target today per the user stop instruction.
- [x] `apps/desktop/src/screens/RecordingStudioScreen.browser.test.tsx`: large
      browser test file. Reassessed as a residual test-organization risk, not a
      new target today per the user stop instruction.
- [x] `apps/desktop/src-tauri/src/recorder/tests.rs`: large recorder test file.
      Reassessed as a residual test-organization risk, not a new target today
      per the user stop instruction.

Latest discovery notes:

- 2026-06-12 closure pass after AI worker execution split:
  - No new refactoring targets were added, per the user stop instruction.
  - `COREPACK_HOME=/tmp/corepack corepack yarn lint` passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn typecheck` passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn test` failed under the default
    sandbox only because Vitest could not bind a loopback port, then passed
    with loopback permission: desktop 28 files / 263 tests, shared 1 file / 9
    tests, ffmpeg 1 file / 1 test.
  - `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings.
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml ai::tests::`
    passed: 44 tests.
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` failed under
    the default sandbox only because transcription tests could not bind
    loopback ports, then passed with loopback permission: 276 tests.
  - Suppression search remains limited to narrow `allow(dead_code)` uses in
    `recorder/errors.rs` and `whisper_backend`.
  - Residual large files are documented rather than added as new targets today:
    large integration/browser tests remain above threshold because they protect
    broad workflows; the largest remaining production files are benchmark,
    storage chunk, model download, activity processing, Linux capture, storage
    limit, GPU compositor, geometry, and controller modules that should be
    reconsidered in a future cleanup session if development returns to those
    areas.
- 2026-06-12 after AI audio processing split:
  - `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml` passed.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings.
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml ai::tests::`
    passed: 44 tests.
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` passed with
    loopback binding permission.
  - Suppression search remains limited to narrow `allow(dead_code)` uses in
    `recorder/errors.rs` and `whisper_backend`.
  - Largest production files are now `recorder/benchmark.rs` 487,
    `storage/ai/chunks.rs` 475, `ai/processing/worker/execution.rs` 474,
    `ai/models/download.rs` 469, `ai/processing/activity.rs` 450, Linux capture
    session 445, `storage_limits.rs` 444, Linux GPU compositor 441,
    `scene-layout/layout-geometry.ts` 425, and library overview controller 415.
  - Added `ai/processing/worker/execution.rs` as the next production refactor
    target because it mixes job-state transitions, source dispatch, chunk
    lifecycle persistence, cancellation reclassification, notification updates,
    and processing-state emission.
- 2026-06-12 after studio timeline store split:
  - `COREPACK_HOME=/tmp/corepack corepack yarn lint` passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn typecheck` passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn test` passed with loopback
    binding permission.
  - Suppression search remains limited to narrow `allow(dead_code)` uses in
    `recorder/errors.rs` and `whisper_backend`.
  - Largest production files are now `ai/processing/audio.rs` 489,
    `recorder/benchmark.rs` 487, `storage/ai/chunks.rs` 475,
    `ai/processing/worker/execution.rs` 474, `ai/models/download.rs` 469,
    `ai/processing/activity.rs` 450, Linux capture session 445,
    `storage_limits.rs` 444, Linux GPU compositor 441, and
    `scene-layout/layout-geometry.ts` 425.
  - Added `ai/processing/audio.rs` as the next production refactor target
    because it mixes FFmpeg extraction, provider calls, silence range planning,
    transcript cleanup, and transcript persistence in one async flow.
- 2026-06-12 after AI settings split: size analysis still ranks `ai.rs`,
  Linux `capture.rs`, `storage/tests.rs`, `transcription.rs`, library, media
  writer, and `storage/ai.rs` as the largest maintainability risks.
- 2026-06-12 `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
  now passes without warnings after small `ai.rs` and `transcription.rs`
  cleanups.
- 2026-06-12 after media writer split:
  - `COREPACK_HOME=/tmp/corepack corepack yarn lint` passed.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings.
  - Suppression search found only narrow `allow(dead_code)` uses in
    `recorder/errors.rs`, `whisper_backend.rs`, and Linux portal code.
  - Largest files now start with `storage/tests.rs` 2,487 lines,
    `RecordingStudioScreen.browser.test.tsx` 2,385, `ai/tests.rs` 1,892,
    `storage/ai.rs` 1,810, `settings.rs` 1,539,
    `recorder-workspace-store.test.ts` 1,378, `recorder/tests.rs` 1,374,
    `ai/processing/visual.rs` 1,284, `ai/processing/worker.rs` 1,244,
    and `scene-layout.test.ts` 1,232.
  - Added `storage/ai.rs` as the next production refactor target. Large test
    files remain near the top and should be split after the production
    ownership boundaries they cover are clearer.
- 2026-06-12 after storage AI split:
  - `COREPACK_HOME=/tmp/corepack corepack yarn lint` passed.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings.
  - Suppression search remained limited to narrow `allow(dead_code)` uses in
    `recorder/errors.rs`, `whisper_backend.rs`, and Linux portal code.
- 2026-06-12 after AI command split:
  - Size and complexity scan ranks production offenders as
    `recorder/commands.rs` 537 lines, `DiagnosticsScreen.tsx` 534,
    `LibraryScreen.tsx` 523, `notifications.rs` 513,
    `studio-timeline-store.ts` 488, `recorder/benchmark.rs` 487,
    `storage/ai/chunks.rs` 475, and `ai/processing/worker/execution.rs` 474.
  - Hook/transition scan flags `LibraryScreen.tsx`,
    `DiagnosticsScreen.tsx`, `studio-timeline-store.ts`, and playback/redaction
    controllers as maintainability risks beyond raw file length.
  - Suppression search remains limited to narrow `allow(dead_code)` uses in
    `recorder/errors.rs` and `whisper_backend`.
  - Added `recorder/commands.rs` as the next production refactor target because
    it mixes command families and lifecycle side effects behind one internal
    implementation module.
  - Largest files now start with `storage/tests.rs` 2,487 lines,
    `RecordingStudioScreen.browser.test.tsx` 2,385, `ai/tests.rs` 1,892,
    `settings.rs` 1,539, `recorder-workspace-store.test.ts` 1,378,
    `recorder/tests.rs` 1,374, `ai/processing/visual.rs` 1,284,
    `ai/processing/worker.rs` 1,244, `scene-layout.test.ts` 1,232,
    and `CaptureScreen.browser.test.tsx` 1,205.
  - Added `settings.rs` as the next production refactor target.
- 2026-06-12 after settings split:
  - `COREPACK_HOME=/tmp/corepack corepack yarn lint` passed.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings.
  - Suppression search remained limited to narrow `allow(dead_code)` uses in
    `recorder/errors.rs`, `whisper_backend.rs`, and Linux portal code.
  - Largest production files are now `ai/processing/visual.rs` 1,284,
    `ai/processing/worker.rs` 1,244, `playback_server.rs` 1,145,
    `ai/models.rs` 1,046, `storage/schema.rs` 865, `ai/results.rs` 819,
    `whisper_backend.rs` 813, and `recorder/capture_runtime.rs` 762.
  - Added `ai/processing/visual.rs` as the next production refactor target.
- 2026-06-12 after visual processing split:
  - `COREPACK_HOME=/tmp/corepack corepack yarn lint` passed.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings.
  - Suppression search remained limited to narrow `allow(dead_code)` uses in
    `recorder/errors.rs`, `whisper_backend.rs`, and Linux portal code.
  - Largest production files are now `ai/processing/worker.rs` 1,244,
    `playback_server.rs` 1,145, `ai/models.rs` 1,046, `storage/schema.rs`
    865, `ai/results.rs` 819, `whisper_backend.rs` 813,
    `recorder/capture_runtime.rs` 762, and `library/waveforms.rs` 716.
  - Added `ai/processing/worker.rs` as the next production refactor target.
- 2026-06-12 after AI processing worker split:
  - `COREPACK_HOME=/tmp/corepack corepack yarn lint` passed.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings.
  - Full `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` passed
    with loopback binding permission: 276 tests.
  - Suppression search remained limited to narrow `allow(dead_code)` uses in
    `recorder/errors.rs`, `whisper_backend.rs`, and Linux portal code.
  - Largest production files are now `playback_server.rs` 1,144,
    `ai/models.rs` 1,045, `storage/schema.rs` 864, `ai/results.rs` 818,
    `whisper_backend.rs` 812, `recorder/capture_runtime.rs` 761,
    `library/waveforms.rs` 715, and `recorder/manager.rs` 712.
  - Added `playback_server.rs` as the next production refactor target.
- 2026-06-12 after playback server split:
  - `COREPACK_HOME=/tmp/corepack corepack yarn lint` passed.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings.
  - Full `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` passed
    with loopback binding permission: 276 tests.
  - Suppression search remained limited to narrow `allow(dead_code)` uses in
    `recorder/errors.rs`, `whisper_backend.rs`, and Linux portal code.
  - Largest production files are now `ai/models.rs` 1,045,
    `storage/schema.rs` 864, `ai/results.rs` 818, `whisper_backend.rs` 812,
    `recorder/capture_runtime.rs` 761, `library/waveforms.rs` 715,
    `recorder/manager.rs` 712, and `diagnostics.rs` 658.
  - Added `ai/models.rs` as the next production refactor target.
- 2026-06-12 after AI model split:
  - `COREPACK_HOME=/tmp/corepack corepack yarn lint` passed.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings.
  - Full `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` passed
    with loopback binding permission: 276 tests.
  - Suppression search remained limited to narrow `allow(dead_code)` uses in
    `recorder/errors.rs`, `whisper_backend.rs`, and Linux portal code.
  - Largest production files are now `storage/schema.rs` 864,
    `ai/results.rs` 818, `whisper_backend.rs` 812,
    `recorder/capture_runtime.rs` 761, `library/waveforms.rs` 715,
    `recorder/manager.rs` 712, `diagnostics.rs` 658,
    and `SettingsScreen.tsx` 637.
  - Added `storage/schema.rs` as the next production refactor target.
- 2026-06-12 after storage schema split:
  - `COREPACK_HOME=/tmp/corepack corepack yarn lint` passed.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings.
  - Full `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` passed
    with loopback binding permission: 276 tests.
  - Suppression search remained limited to narrow `allow(dead_code)` uses in
    `recorder/errors.rs`, `whisper_backend.rs`, and Linux portal code.
  - Largest production files are now `ai/results.rs` 818,
    `whisper_backend.rs` 812, `recorder/capture_runtime.rs` 761,
    `library/waveforms.rs` 715, `recorder/manager.rs` 712,
    `diagnostics.rs` 658, `SettingsScreen.tsx` 637,
    and Linux `portal.rs` 622.
  - Added `ai/results.rs` as the next production refactor target.
- 2026-06-12 after AI results split:
  - `COREPACK_HOME=/tmp/corepack corepack yarn lint` passed.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings.
  - Full `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` passed
    with loopback binding permission: 276 tests.
  - Suppression search remained limited to narrow `allow(dead_code)` uses in
    `recorder/errors.rs`, `whisper_backend.rs`, and Linux portal code.
  - Largest production files are now `whisper_backend.rs` 812,
    `recorder/capture_runtime.rs` 761, `library/waveforms.rs` 715,
    `recorder/manager.rs` 712, `diagnostics.rs` 658,
    `SettingsScreen.tsx` 637, Linux `portal.rs` 622,
    and `recorder/compositor.rs` 609.
  - Added `whisper_backend.rs` as the next production refactor target.
- 2026-06-12 after Whisper backend split:
  - `COREPACK_HOME=/tmp/corepack corepack yarn lint` passed.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings.
  - Full `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` passed
    with loopback binding permission: 276 tests.
  - Suppression search now finds narrow dead-code allows in generated
    whisper.cpp bindings, documented Whisper integration hooks,
    `recorder/errors.rs`, and Linux portal code.
  - Largest production files are now `recorder/capture_runtime.rs` 761,
    `library/waveforms.rs` 715, `recorder/manager.rs` 712,
    `diagnostics.rs` 658, `SettingsScreen.tsx` 637, Linux `portal.rs` 622,
    `recorder/compositor.rs` 609, library studio playback controller 601,
    library studio sidebar 596, and library track lane 580.
  - Added `recorder/capture_runtime.rs` as the next production refactor target.
- 2026-06-12 after capture runtime split:
  - `COREPACK_HOME=/tmp/corepack corepack yarn lint` passed.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings.
  - Full `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` passed
    with loopback binding permission: 276 tests.
  - Suppression search remains limited to narrow dead-code allows in generated
    whisper.cpp bindings, documented Whisper integration hooks,
    `recorder/errors.rs`, and Linux portal code.
  - Largest production files are now `library/waveforms.rs` 715,
    `recorder/manager.rs` 712, `diagnostics.rs` 658,
    `SettingsScreen.tsx` 637, Linux `portal.rs` 622,
    `recorder/compositor.rs` 609, library studio playback controller 601,
    library studio sidebar 596, library track lane 580, and
    `RecordingStudioScreen.tsx` 577.
  - Added `library/waveforms.rs` as the next production refactor target.
- 2026-06-12 after library waveform split:
  - `COREPACK_HOME=/tmp/corepack corepack yarn lint` passed.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings.
  - Full `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` passed
    with loopback binding permission: 276 tests.
  - Suppression search remains limited to narrow dead-code allows in generated
    whisper.cpp bindings, documented Whisper integration hooks,
    `recorder/errors.rs`, and Linux portal code.
  - Largest production files are now `recorder/manager.rs` 712,
    `diagnostics.rs` 658, `SettingsScreen.tsx` 637, Linux `portal.rs` 622,
    `recorder/compositor.rs` 609, library studio playback controller 601,
    library studio sidebar 596, library track lane 580,
    `RecordingStudioScreen.tsx` 577, and recorder workspace store 570.
  - Added `recorder/manager.rs` as the next production refactor target.
- 2026-06-12 after recorder manager split:
  - `COREPACK_HOME=/tmp/corepack corepack yarn lint` passed.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings.
  - Full `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` passed
    with loopback binding permission: 276 tests.
  - Suppression search remains limited to narrow dead-code allows in generated
    whisper.cpp bindings, documented Whisper integration hooks,
    `recorder/errors.rs`, and Linux portal code.
  - Largest production files are now `diagnostics.rs` 658,
    `SettingsScreen.tsx` 637, Linux `portal.rs` 622,
    `recorder/compositor.rs` 609, library studio playback controller 601,
    library studio sidebar 596, library track lane 580,
    `RecordingStudioScreen.tsx` 577, recorder workspace store 570, and track
    timeline 563.
  - Added `diagnostics.rs` as the next production refactor target.
- 2026-06-12 after diagnostics split:
  - `COREPACK_HOME=/tmp/corepack corepack yarn lint` passed.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings.
  - Full `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` passed
    with loopback binding permission: 276 tests.
  - Suppression search remains limited to narrow dead-code allows in generated
    whisper.cpp bindings, documented Whisper integration hooks,
    `recorder/errors.rs`, and Linux portal code.
  - Largest production files are now `SettingsScreen.tsx` 637,
    Linux `portal.rs` 622, `recorder/compositor.rs` 609, library studio
    playback controller 601, library studio sidebar 596, library track lane
    580, `RecordingStudioScreen.tsx` 577, recorder workspace store 570, track
    timeline 563, and `ai/commands.rs` 542.
  - Added `SettingsScreen.tsx` as the next production refactor target.
- 2026-06-12 after settings screen split:
  - `COREPACK_HOME=/tmp/corepack corepack yarn lint` passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn typecheck` passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn test` passed with loopback
    binding permission.
  - Suppression search remains limited to narrow dead-code allows in generated
    whisper.cpp bindings, documented Whisper integration hooks,
    `recorder/errors.rs`, and Linux portal code.
  - Largest production files are now Linux `portal.rs` 622,
    `recorder/compositor.rs` 609, library studio playback controller 601,
    library studio sidebar 596, library track lane 580,
    `RecordingStudioScreen.tsx` 577, recorder workspace store 570, track
    timeline 563, `ai/commands.rs` 542, and `recorder/commands.rs` 537.
  - Added Linux `portal.rs` as the next production refactor target.
- 2026-06-12 after Linux portal split:
  - `COREPACK_HOME=/tmp/corepack corepack yarn lint` passed.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings.
  - Full `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` passed
    with loopback binding permission: 276 tests.
  - Suppression search is now limited to narrow dead-code allows in generated
    whisper.cpp bindings, documented Whisper integration hooks, and
    `recorder/errors.rs`.
  - Largest production files are now `recorder/compositor.rs` 609, library
    studio playback controller 601, library studio sidebar 596, library track
    lane 580, `RecordingStudioScreen.tsx` 577, recorder workspace store 570,
    track timeline 563, `ai/commands.rs` 542, `recorder/commands.rs` 537, and
    `DiagnosticsScreen.tsx` 534.
  - Added `recorder/compositor.rs` as the next production refactor target.
- 2026-06-12 after recorder compositor split:
  - `COREPACK_HOME=/tmp/corepack corepack yarn lint` passed.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings.
  - Full `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` passed
    with loopback binding permission: 276 tests.
  - Suppression search remains limited to narrow dead-code allows in generated
    whisper.cpp bindings, documented Whisper integration hooks, and
    `recorder/errors.rs`.
  - Largest production files are now library studio playback controller 601,
    library studio sidebar 596, library track lane 580,
    `RecordingStudioScreen.tsx` 577, recorder workspace store 570, track
    timeline 563, `ai/commands.rs` 542, `recorder/commands.rs` 537,
    `DiagnosticsScreen.tsx` 534, and `LibraryScreen.tsx` 523.
  - Added library studio playback controller as the next production refactor
    target.
- 2026-06-12 after library studio playback controller and sidebar splits:
  - `COREPACK_HOME=/tmp/corepack corepack yarn lint` passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn typecheck` passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn test` passed with loopback
    binding permission.
  - Suppression search remains limited to narrow dead-code allows in
    documented Whisper integration hooks and `recorder/errors.rs`.
  - Largest production files are now library track lane 580,
    `RecordingStudioScreen.tsx` 577, shared `ai.ts` 571, recorder workspace
    store 570, track timeline 563, `ai/commands.rs` 542,
    `recorder/commands.rs` 537, `DiagnosticsScreen.tsx` 534,
    `LibraryScreen.tsx` 523, and `notifications.rs` 513.
  - Added library studio track lane as the next production refactor target.
- 2026-06-12 after library studio track lane split:
  - `COREPACK_HOME=/tmp/corepack corepack yarn lint` passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn typecheck` passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn test` passed with loopback
    binding permission.
  - Suppression search remains limited to narrow dead-code allows in documented
    Whisper integration hooks and `recorder/errors.rs`.
  - Largest production files are now `RecordingStudioScreen.tsx` 577, shared
    `ai.ts` 571, recorder workspace store 570, track timeline 563,
    `ai/commands.rs` 542, `recorder/commands.rs` 537,
    `DiagnosticsScreen.tsx` 534, `LibraryScreen.tsx` 523,
    `notifications.rs` 513, and `ai/processing/audio.rs` 489.
  - Added `RecordingStudioScreen.tsx` as the next production refactor target.
- 2026-06-12 after recording studio route split:
  - `COREPACK_HOME=/tmp/corepack corepack yarn lint` passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn typecheck` passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn test` passed with loopback
    binding permission.
  - Suppression search remains limited to narrow dead-code allows in documented
    Whisper integration hooks and `recorder/errors.rs`.
  - Largest production files are now shared `ai.ts` 571, recorder workspace
    store 570, track timeline 563, `ai/commands.rs` 542,
    `recorder/commands.rs` 537, `DiagnosticsScreen.tsx` 534,
    `LibraryScreen.tsx` 523, `notifications.rs` 513,
    `ai/processing/audio.rs` 489, and studio timeline store 488.
  - Added shared `ai.ts` as the next production refactor target.
- 2026-06-12 after shared AI contract split:
  - `COREPACK_HOME=/tmp/corepack corepack yarn lint` passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn typecheck` passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn test` passed with loopback
    binding permission.
  - Suppression search remains limited to narrow dead-code allows in documented
    Whisper integration hooks and `recorder/errors.rs`.
  - Largest production files are now recorder workspace store 570, track
    timeline 563, `ai/commands.rs` 542, `recorder/commands.rs` 537,
    `DiagnosticsScreen.tsx` 534, `LibraryScreen.tsx` 523,
    `notifications.rs` 513, `ai/processing/audio.rs` 489, studio timeline
    store 488, and recorder benchmark 487.
  - Added recorder workspace store as the next production refactor target.
- 2026-06-12 after recorder workspace store split:
  - `COREPACK_HOME=/tmp/corepack corepack yarn lint` passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn typecheck` passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn test` passed with loopback
    binding permission.
  - Suppression search remains limited to narrow dead-code allows in documented
    Whisper integration hooks and `recorder/errors.rs`.
  - Largest production files are now track timeline 563, `ai/commands.rs` 542,
    `recorder/commands.rs` 537, `DiagnosticsScreen.tsx` 534,
    `LibraryScreen.tsx` 523, `notifications.rs` 513,
    `ai/processing/audio.rs` 489, studio timeline store 488, recorder
    benchmark 487, and storage AI chunks 475.
  - Added track timeline as the next production refactor target.
- 2026-06-12 after track timeline split:
  - `COREPACK_HOME=/tmp/corepack corepack yarn lint` passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn typecheck` passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn test` passed with loopback
    binding permission.
  - Suppression search remains limited to narrow dead-code allows in documented
    Whisper integration hooks and `recorder/errors.rs`.
  - Largest production files are now `ai/commands.rs` 542,
    `recorder/commands.rs` 537, `DiagnosticsScreen.tsx` 534,
    `LibraryScreen.tsx` 523, `notifications.rs` 513,
    `ai/processing/audio.rs` 489, studio timeline store 488, recorder
    benchmark 487, storage AI chunks 475, and AI worker execution 474.
  - Added AI Tauri commands as the next production refactor target.
- 2026-06-12 after library overview, diagnostics screen, and notification
  splits:
  - `COREPACK_HOME=/tmp/corepack corepack yarn lint` passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn typecheck` passed.
  - `COREPACK_HOME=/tmp/corepack corepack yarn test` passed with loopback
    binding permission.
  - `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
    passed without warnings.
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` passed with
    loopback binding permission.
  - Suppression search remains limited to narrow `allow(dead_code)` uses in
    `recorder/errors.rs` and `whisper_backend`.
  - Largest production files are now `ai/processing/audio.rs` 489,
    `studio-timeline-store.ts` 488, `recorder/benchmark.rs` 487,
    `storage/ai/chunks.rs` 475, `ai/processing/worker/execution.rs` 474,
    `ai/models/download.rs` 469, `ai/processing/activity.rs` 450,
    Linux capture session 445, `storage_limits.rs` 444, and Linux GPU
    compositor 441.
  - Added `studio-timeline-store.ts` as the next production refactor target
    because its selection/range reducers are central to timeline editing and
    redaction behavior.
- 2026-06-12 suppression search found broad dead-code allows in
  `transcription.rs`, Linux `capture.rs`, `recorder/capture_runtime.rs`, and
  `recorder/compositor.rs`, plus narrower dead-code allows in
  `whisper_backend.rs`, `recorder/errors.rs`, and Linux portal code.
- 2026-06-12 after Linux capture and library splits: size analysis now ranks
  `storage/tests.rs`, `RecordingStudioScreen.browser.test.tsx`, `ai/tests.rs`,
  `recorder/media_writer.rs`, `storage/ai.rs`, `settings.rs`,
  `recorder/tests.rs`, `ai/processing/visual.rs`, `ai/processing/worker.rs`,
  `playback_server.rs`, and `recorder/capture_runtime.rs` as the largest files.
  Suppression search now finds broad allows only in
  `recorder/capture_runtime.rs` and `recorder/compositor.rs`; narrower
  dead-code allows remain in `whisper_backend.rs`, Linux portal code, and
  `recorder/errors.rs`.

## Final Completion Gate

Do not mark the active goal complete until all of these are true.

- [x] Every task above is checked or explicitly replaced by a documented better
      task.
- [x] No known source file remains above the repo's maintainability threshold
      without a documented reason.
- [x] No broad dead-code or lint suppressions remain without a narrow technical
      justification.
- [x] `corepack yarn lint` passes.
- [x] `corepack yarn typecheck` passes.
- [x] `corepack yarn test` passes.
- [x] `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets`
      passes without actionable warnings.
- [x] `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` passes.
- [x] The final handoff reports remaining risks, if any.
