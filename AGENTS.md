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
record a full working day, process eligible local chunks automatically, extract
searchable timeline annotations, task/todo candidates, conversation summaries,
and a daily digest, then let the user cut or hide sensitive spans with derived
data invalidated for those spans.

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
  and manual scissors/redaction after processing.

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
- The app MUST provide a scissors/redaction workflow so users can remove
  sensitive spans after capture and AI processing; OCR, transcripts, VLM output,
  embeddings, summaries, and search data for removed spans MUST be deleted or
  invalidated.
- Capture and processing SHOULD be decoupled. A full day can be recorded first
  and processed later, but future processing SHOULD also detect eligible chunks
  during active recording and catch up toward realtime when local capacity
  permits.

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
- Redaction state MUST be part of processing and reprocessing eligibility. Do
  not process spans that the user has already cut or hidden, and invalidate
  derived data when a span is cut or hidden after processing.
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
- Route and screen files SHOULD stay thin. They SHOULD own route params,
  top-level loading/error/delete flows, and feature composition; they SHOULD NOT
  accumulate detailed player, editor, timeline, or panel implementation.
- Feature-specific UI SHOULD live under `apps/desktop/src/features/<feature>/`
  once it has more than one focused responsibility. Avoid parking complex
  feature components in `apps/desktop/src/screens`.
- Large UI state shared by sibling components SHOULD be owned by a named hook or
  controller object, not by mutable command refs between siblings. Mutable refs
  are acceptable for native imperative boundaries such as media elements,
  canvas, observers, timers, and scroll containers.
- Component props SHOULD be grouped by responsibility when the surface grows:
  prefer focused `model`/`actions`/`status` objects or slot composition over long
  mixed lists of unrelated values and callbacks.
- Component names SHOULD describe responsibility precisely: use `*Screen` only
  for routes, `*Layout` for layout-only composition, `*Panel` for side panels,
  `*Controls` for action clusters, and `use*Controller` for stateful
  orchestration.

## Complexity, Performance & Refactoring Hygiene

- Agents MUST treat file size as only one maintainability signal. They MUST also
  watch for high branching, deeply nested conditionals, long functions, mixed
  sync/async control flow, excessive hook state, broad dependency arrays,
  repeated data derivation, render-time heavy work, unclear ownership, and tests
  that require large unrelated setup.
- Agents MUST NOT add substantial behavior to a file or module that is already
  large, complex, or ownership-confused without first checking whether the
  behavior belongs in a focused module. As a rule of thumb, production files
  above roughly 400 lines SHOULD be treated as split candidates, and files above
  roughly 500 lines MUST have a clear ownership justification or be split before
  more behavior is added.
- Agents SHOULD split earlier than the line-count threshold when complexity is
  already high. A 150-line hook with fetch lifecycle, timers, subscriptions,
  mutable refs, retries, and UI state can be a worse offender than a longer
  declarative component.
- Agents MUST split by durable ownership, not by arbitrary line count. Good
  split boundaries include route shell vs feature controller, toolbar vs dialog,
  store composition vs action groups, IPC commands vs mapping/validation,
  provider adapters vs domain logic, and public contract barrel vs domain
  contract modules.
- Agents SHOULD reduce cognitive complexity before adding features. Prefer
  named predicates, small deterministic helpers, focused reducers, typed state
  machines, and explicit action objects over nested conditionals and scattered
  boolean flags.
- Route files in `apps/desktop/src/screens` MUST stay composition-focused.
  They SHOULD own route params, navigation, top-level loading/error/delete
  flows, and feature assembly only. Data loading, dialogs, toolbar actions,
  playback control, editor logic, and debug workflows SHOULD live under
  `apps/desktop/src/features/<feature>/`.
- React hooks SHOULD have one clear ownership reason to exist. Hooks that mix
  data loading, subscriptions, timers, DOM/media refs, derived view models,
  mutations, and error handling SHOULD be split into a controller plus focused
  helpers.
- React components SHOULD avoid repeated expensive derivations during render.
  Agents SHOULD precompute with focused selectors, memoized view models, or
  deterministic helpers when data volume or render frequency makes repeated work
  meaningful. Do not add `useMemo` by reflex; use it when it protects a real
  allocation, calculation, or referential-stability boundary.
- Agents MUST keep effect dependencies honest. Do not silence dependency
  warnings or hide values in refs to avoid re-running an effect unless the ref is
  truly modeling an imperative boundary. Effects SHOULD synchronize with
  external systems; pure derivations SHOULD stay out of effects.
- Zustand stores MUST NOT become catch-all modules. Store files SHOULD define
  the public hook and compose focused action groups. State types, initial state,
  persistence/load/save actions, preview actions, lifecycle actions, selection
  actions, and deterministic reducers SHOULD live in separate modules once more
  than one ownership area exists.
- Shared package files SHOULD NOT collect unrelated contracts. Keep
  `packages/shared/src/*.ts` files as compatibility barrels when needed, but
  move settings, commands, processing state, notifications, transcript, visual,
  recorder, and diagnostics contracts into focused modules.
- Rust modules SHOULD keep Tauri command handlers thin. Commands SHOULD validate
  input, call focused domain/storage/service modules, and map errors. Long files
  mixing commands, SQL rows, filesystem work, background jobs, and DTO mapping
  SHOULD be split before adding behavior.
- Compatibility boundaries SHOULD be preserved during refactors. Existing
  public hooks, route components, IPC command names, shared package exports, and
  Rust module imports SHOULD keep working unless the task explicitly includes a
  breaking change.
- Performance-sensitive paths SHOULD be identified before refactoring. Timeline
  rendering, canvas interaction, playback synchronization, recorder preview
  frames, capture loops, processing queues, storage scans, and large test
  fixtures SHOULD avoid unnecessary cloning, repeated filtering over large
  arrays, unstable callbacks passed to dense child trees, and avoidable
  re-renders.
- Agents SHOULD prefer data structures that match access patterns. Use maps,
  sets, indexes, and pre-grouped view models when repeated lookup/filter work is
  part of a hot path; keep simple arrays when data is small and clarity wins.
- When a refactor checklist exists, especially `docs/refactoring-tasks.md`,
  agents SHOULD update it before and after each major cleanup: document the
  offender, ownership problem, complexity/performance risk, planned split,
  verification commands, current line counts when relevant, and the next
  discovered offender. Do not mark a checklist item complete until the split is
  implemented and verified.
- New abstractions MUST reduce real ownership confusion or duplication. Do not
  create wrapper modules that only move code around without making responsibility
  clearer.

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
- Components approaching the repository `max-lines` lint warning SHOULD be split
  before adding more behavior. Do not silence the warning unless a generated or
  framework-constrained file leaves no practical alternative.
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
- Large test files SHOULD also be split by workflow once production ownership
  boundaries are clear. Prefer scenario-focused test modules such as playback,
  redaction, route lifecycle, preview lifecycle, scene editing, storage
  migrations, and processing recovery over one monolithic test file.
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
