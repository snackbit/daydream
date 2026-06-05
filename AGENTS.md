# Repository Guidelines

## Communication Style

- You MUST be blunt, direct, and easy to understand.
- You MUST avoid filler, corporate phrasing, and vague reassurance.
- You MUST stay technically precise.

## Daydream Overview

Daydream is a local-first desktop recall assistant. Its job is to help a user
answer "what did I do today?" from private screen, audio, activity, and window
context recorded on hardware the user controls.

The intended product is an on-demand Tauri or Electron desktop app that can
record a full working day, let the user cut or hide sensitive spans before
processing, then extract searchable timeline annotations, task/todo candidates,
conversation summaries, and a daily digest.

## RFC 2119 Language

All instructions in this document use RFC-style terms. MUST, MUST NOT, SHOULD,
SHOULD NOT, and MAY must be interpreted accordingly.

## Current Repository State

- This repository is a Yarn 4 monorepo.
- The desktop app lives in `apps/desktop` and uses Tauri 2, React, Vite,
  Tailwind, shadcn-style components, TanStack Router, and Zustand.
- Shared TypeScript config, lint config, diagnostics types, and the development
  FFmpeg resolver live in `packages/`.
- SQLite persistence is owned by the Rust/Tauri backend and initialized on app
  startup.
- Real capture, redaction, OCR, transcription, VLM inference, vector indexing,
  and timeline playback are not implemented yet.

## Engineering Approach

- You MUST solve root causes instead of adding workaround branches.
- You MUST verify important assumptions with repo evidence, targeted commands,
  package metadata, or official docs before depending on them.
- You SHOULD prefer straightforward logic with clear invariants over
  fallback-heavy implementations.
- You MUST keep privacy, local execution, and explicit user control central to
  every architecture decision.
- You SHOULD keep processing steps deterministic and resumable where practical,
  because full-day recordings will be expensive to process.
- You MUST document non-obvious decisions, native permission requirements,
  storage formats, and model/tool tradeoffs as they are introduced.
- You MUST use Yarn via Corepack for JavaScript tooling.
- You SHOULD run `yarn lint:fix` frequently while editing. In this repo,
  ESLint is the main formatting and code-smell gate.

## Target Architecture Direction

- Desktop shell: Tauri 2 is the current scaffold choice.
- Package manager: Yarn 4 stable through Corepack.
- Capture tooling: FFmpeg SHOULD be embedded, vendored, or otherwise managed by
  the app distribution so users are not expected to install it manually. The
  current dev scaffold resolves FFmpeg through `ffmpeg-static`.
- Local metadata store: SQLite.
- Local vector/search index: LanceDB or Qdrant. Keep the index boundary isolated
  so the backend can switch between these without rewriting capture or timeline
  logic.
- AI processing: Whisper-style transcription, OCR on retained frames, VLM
  inference for visual context, and LLM chunk summarization/digest generation.
- Storage: local video/audio/object store with explicit retention and deletion
  semantics.
- UI: timeline playback, searchable annotations, source-linked digest entries,
  and manual scissors/redaction before processing.

## Privacy & Security

- You MUST treat screen recordings, audio, OCR text, transcripts, focused window
  titles, open-window lists, keyboard/mouse activity, and generated summaries as
  highly sensitive personal data.
- You MUST NOT upload captured data, derived text, embeddings, or summaries to a
  remote service unless the user has explicitly enabled that capability and the
  code path is clearly labeled.
- You MUST NOT log raw frames, raw OCR text, transcripts, window titles, audio
  content, embeddings, or model prompts by default.
- You SHOULD prefer local models and local indexing for baseline operation.
- You MUST design deletion as real deletion: if the user cuts or removes a span,
  derived frames, audio chunks, OCR, transcripts, embeddings, and summaries for
  that span must also be removed or invalidated.
- You SHOULD make privacy-sensitive operations auditable through local metadata
  without exposing private content in logs.

## Capture Pipeline Expectations

- The app SHOULD support multi-screen recording.
- The app SHOULD capture both desktop audio and microphone audio, keeping the
  streams distinguishable during processing.
- The default recording concept is roughly 1 FPS with high visual quality and
  FFmpeg tuning that balances OCR accuracy, storage size, and write rate.
- The app SHOULD capture keyboard and mouse activity metadata sufficient to
  infer whether the user was actively using the computer.
- The app SHOULD capture focused window identity, focused window title, and open
  window inventory where the OS permits it.
- The app MUST provide a pre-processing scissors/redaction workflow so users can
  remove sensitive spans before OCR, transcription, VLM, embeddings, or LLM
  summarization run.
- Capture and processing SHOULD be decoupled so a full day can be recorded first
  and processed later.

## Processing Pipeline Expectations

- Processing SHOULD be chunk-based and resumable.
- Frame extraction SHOULD preserve enough quality for accurate OCR and UI text
  extraction.
- OCR SHOULD run on every retained frame unless a later task explicitly defines
  a smarter sampling or deduplication policy.
- Whisper transcription SHOULD align desktop and microphone audio with timeline
  timestamps.
- VLM inference SHOULD operate on bounded visual chunks and produce
  source-linked observations, not free-floating claims.
- LLM summarization SHOULD consume chunked OCR, transcript, VLM output, activity
  signals, and window metadata to produce daily digest sections and task/todo
  candidates.
- Generated tasks, todos, and "I have been working on..." annotations MUST keep
  timestamp provenance so the UI can jump back to source evidence.

## Data Model & Indexing Guidance

- SQLite SHOULD own durable relational metadata: recordings, spans, chunks,
  frame references, audio references, OCR segments, transcript segments,
  activity samples, window events, annotations, redactions, and processing job
  status.
- SQLite access MUST stay in the Rust/Tauri backend. React MUST use typed Tauri
  commands instead of opening or querying SQLite directly.
- SQLite migrations MUST be explicit and tested. The current backend uses
  embedded Rust migrations.
- LanceDB or Qdrant SHOULD own vector search data behind a narrow adapter.
- Local files SHOULD be content-addressed or otherwise stable enough that
  processing jobs can resume without losing provenance.
- Redaction state MUST be part of processing eligibility. Do not process spans
  that the user has cut or hidden.
- Search results and digest items SHOULD link back to timestamps, source type,
  and confidence where available.

## Desktop & Native Integration

- You MUST be explicit about OS permissions for screen recording, microphone
  capture, accessibility/input monitoring, and window metadata.
- You SHOULD isolate native capture integrations from domain processing logic.
- You MUST avoid hidden background capture behavior. Recording state should be
  visible and user-controlled.
- You SHOULD design platform-specific capture code behind typed interfaces so
  Windows, macOS, and Linux differences do not leak through the whole app.

## Coding Style & Boundaries

- TypeScript SHOULD be the default application language once the app is
  scaffolded.
- React UI code MUST use the shadcn/Tailwind baseline in `apps/desktop`.
  "Baseline" means actual shadcn components where available, not visually
  similar hand-rolled markup.
- UI navigation MUST go through TanStack Router.
- Local UI state SHOULD use Zustand when component-local state is not enough.
- You SHOULD use strict typing at external boundaries: native capture payloads,
  processing jobs, model outputs, persisted records, and IPC/API contracts.
- You MUST validate untrusted or model-generated data at runtime before
  persisting or displaying it as structured facts.
- You SHOULD keep side effects at edges: capture adapters, FFmpeg wrappers,
  model runners, filesystem storage, and database adapters.
- You SHOULD keep domain logic deterministic and testable: chunk planning,
  redaction eligibility, timeline alignment, annotation merging, and digest
  construction.
- You MUST NOT couple UI components directly to FFmpeg, model runners, or raw
  database access.

## UI Component Quality

- React UI code MUST use actual shadcn components from
  `apps/desktop/src/components/ui` when an eligible component exists.
- Before building a new UI surface, agents MUST inspect existing shadcn
  components and the shadcn docs for needed components.
- If a needed shadcn component is missing, agents SHOULD add it through the
  local `components.json` setup instead of hand-rolling equivalent controls.
- Raw `input`, `select`, `checkbox`, alert banners, cards, scroll containers,
  separators, tabs, resizable panes, and tooltip patterns SHOULD NOT be
  hand-rolled when shadcn provides the component.
- Large route/page components SHOULD be split into focused components by
  responsibility: data binding, layout shell, toolbar/actions, panels, forms,
  canvas/editor surfaces, and error display.
- UI feature work is not complete until component consistency, accessibility
  labels, responsive layout, and critical UI tests are covered.

## User-Facing Work Quality Bar

- Before implementing user-facing UI, agents MUST identify the primary user
  workflow, the expected visible outcome, and the loading, empty, error,
  disabled, and success states that can occur.
- UI work MUST be validated through real user interactions where practical:
  clicking, typing, selecting, navigating, saving, retrying, and recovering from
  errors.
- Agents MUST NOT treat internal state assertions as a substitute for visible UI
  behavior when a user-visible outcome exists.
- Layout-changing UI work SHOULD be checked at one desktop viewport and one
  narrow viewport. Text must not overlap, truncate critical labels, or make
  primary actions ambiguous.
- Critical controls MUST have accessible names and keyboard-reachable behavior
  unless the control is genuinely pointer-only, such as canvas drag handles.
- Disabled or unavailable primary actions SHOULD make the blocking condition
  visible through nearby status, copy, or error state.

## Dependency & Tooling Hygiene

- You MUST use `npm view <package-name>` or official package metadata before
  adding or upgrading npm dependencies.
- You SHOULD check official docs for Tauri, Electron, FFmpeg packaging, Whisper
  bindings, OCR engines, LanceDB, Qdrant, and native capture libraries before
  choosing an integration.
- You MUST document upgrade-sensitive native dependencies and binary packaging
  decisions.
- You SHOULD keep vendor/tool integrations behind local adapters when API churn,
  platform differences, or lock-in risk is meaningful.

## Testing Guidance

- You SHOULD add tests with the behavior they protect. Do not defer all testing
  to final integration work.
- Tests MUST be scenario-first for user-facing, stateful, native-boundary, or
  privacy-sensitive changes. Start by naming the real workflow and failure mode
  the test protects.
- Interactive React UI changes MUST include Browser Mode or equivalent
  user-level tests for at least one critical workflow unless the agent documents
  why that layer is not practical.
- Unit tests SHOULD cover deterministic logic, data mapping, reducers/stores,
  validation, and edge-case math. They MUST NOT be the only coverage for a
  feature whose main risk is broken user workflow.
- Mocks SHOULD stop at process, native, filesystem, model, network, or Tauri IPC
  boundaries. Do not mock the component, store transition, or domain behavior
  that the test claims to validate.
- UI workflow tests SHOULD prefer visible assertions and accessible queries over
  implementation details. Store/internal assertions MAY support a test but
  SHOULD NOT be the main proof.
- For stateful UI, tests SHOULD cover meaningful combinations: empty and
  populated data, saved and dirty state, loading and error state, unavailable
  permissions/tools, and active lifecycle states.
- A bug found during manual verification SHOULD be converted into a regression
  test before or alongside the fix. If that is not practical, the final handoff
  MUST say why and list the residual manual risk.
- Do not skip tests because setup requires more work. If a test dependency or
  runtime is justified, verify package metadata and add it cleanly.
- Final handoff for non-trivial changes MUST report scenario coverage, commands
  run, manual checks performed or skipped, and important untested risks.
- You SHOULD run `yarn lint:fix`, `yarn lint`, `yarn typecheck`, and
  `yarn test` before handing off TypeScript changes.
- You SHOULD run `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml`
  for Rust/Tauri changes when Linux Tauri system prerequisites are installed.
- Capture tests SHOULD cover multi-screen metadata, audio stream separation,
  permission failures, and recording lifecycle state.
- Redaction tests MUST cover removal of derived data and processing exclusion.
- Processing tests SHOULD cover chunk boundaries, timestamp alignment, OCR and
  transcript provenance, resumability, and partial failure handling.
- Index/search tests SHOULD cover source-linked results and deletion or
  invalidation after redaction.
- UI tests SHOULD cover timeline playback, annotation search, digest source
  navigation, and scissors/redaction workflows.
- Privacy tests SHOULD assert that sensitive raw content is not logged or sent
  to remote services by default.

## Optional Milestone Workflow

- Curated agent runbooks MAY live in `.agents/runbooks/`.
- The milestone workflow is optional and explicit. Agents MUST treat ordinary
  implementation requests as ad-hoc work unless the user mentions a milestone,
  gate, scratchpad, or approved milestone-style plan.
- When milestone-scoped work is explicitly requested, agents MUST follow
  `.agents/runbooks/planning.md`.
- Milestone artifacts SHOULD use `docs/milestone-template.md` and the artifact
  pair `docs/milestones/m<nn>-<slug>.md` plus
  `docs/milestones/m<nn>-<slug>-scratchpad.md`.

## Agent Expectations

- You MUST behave like an expert in TypeScript, desktop app architecture,
  native capture constraints, local AI pipelines, and privacy-sensitive product
  design.
- You SHOULD explore before editing when repository truth is uncertain.
- You MUST keep edits scoped to the user's request.
- You MUST report unusual findings, risky assumptions, and follow-up work that
  matters.
- You SHOULD propose slim updates to this file when new repo conventions become
  stable.
- You MUST NOT create git commits unless the user explicitly asks for one.
