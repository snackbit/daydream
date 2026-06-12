# Milestone M3: Day Index MVP

## Goal

Turn Daydream's M2 evidence layer into the first usable private day index:
after a recording's AI processing completes, Daydream builds searchable,
source-linked day/session/follow-up records, shows a dashboard-first Library
detail view, and lets the user search the indexed day while retaining the
playback studio as the evidence and redaction surface.

## Summary

- M3 implements the technical foundation for `docs/wip-library-day-index-design.md`.
- The Library stops being only a recording picker. It gains configurable
  projections over one local index: days, recordings, sessions, and segments.
- M3 adds a local LanceDB vector store, SQLite FTS metadata, OpenAI-compatible
  provider registry, downstream indexing stages, derived sessions, internal
  follow-up suggestions, and hybrid search.
- M3 deliberately ships search results, not generated chat answers. Cited Q&A
  over the day index is deferred.
- M3 keeps raw playback in the Evidence surface. The default detail experience
  becomes the day dashboard when indexed data exists.

## Inputs

- `AGENTS.md`
- `README.md`
- `.agents/runbooks/planning.md`
- `docs/milestone-template.md`
- `docs/wip-library-day-index-design.md`
- `docs/milestones/m2-ai-processing-foundation.md`
- `docs/milestones/m2-ai-processing-foundation-scratchpad.md`
- `docs/milestones/m2.1-track-timeline-editor.md`
- Existing shared AI, library, settings, and notification types in
  `packages/shared/src/`
- Existing Rust/Tauri storage, settings, AI processing, transcription,
  notification, and Library command surfaces under `apps/desktop/src-tauri/src/`
- Existing Library overview and Recording Studio UI under
  `apps/desktop/src/features/library/`
- LanceDB documentation:
  <https://docs.lancedb.com/>
- LanceDB SDK reference:
  <https://docs.lancedb.com/api-reference>
- OpenAI embeddings API shape:
  <https://platform.openai.com/docs/api-reference/embeddings/create>
- OpenAI Responses API shape:
  <https://platform.openai.com/docs/api-reference/responses/create>

## Locked Decisions

- Milestone artifacts:
  - `docs/milestones/m3-day-index-mvp.md`
  - `docs/milestones/m3-day-index-mvp-scratchpad.md`
- M3 is the Day Index MVP, not another playback-only milestone.
- M3 depends on the M2 evidence model: transcripts, OCR regions, VLM
  observations, activity intervals, processing chunks, jobs, redactions, and
  recording metadata.
- M3 extends the user-facing AI processing flow with downstream durable stages
  for embeddings, FTS indexing, sessionization, day summary generation,
  follow-up extraction, and search index writes.
- The existing `ai-processing` job remains the main user-facing processing
  concept. M3 may add internal downstream jobs/chunks, but Library should
  present them as part of the same processing pipeline unless the user needs
  failure details.
- Indexing starts automatically after the relevant evidence stages complete.
  A user may still manually retry or rebuild the index for a day/recording.
- M3 uses LanceDB as the local vector store.
- LanceDB runs embedded against a local app-data path. No managed or remote
  LanceDB service is part of M3.
- SQLite remains the durable relational owner for index metadata, source
  provenance, user overrides, provider settings, job/chunk state, and FTS rows.
- SQLite access remains Rust-owned. React uses typed Tauri commands.
- Hybrid search means SQLite FTS plus LanceDB vector search plus optional LLM
  reranking.
- Text embeddings are required for M3 closeout.
- Image embeddings are planned behind adapter/table boundaries, but working
  image embeddings are not required for M3 closeout.
- OpenAI-compatible embeddings use `/v1/embeddings`.
- OpenAI-compatible generation uses `/v1/responses` as the required M3
  compatibility floor.
- Local LLM and embedding support means a user-configured external local
  OpenAI-compatible endpoint. Daydream does not bundle, download, or launch a
  local LLM runtime in M3.
- Provider configuration becomes a registry: users define OpenAI-compatible
  providers once, then role settings reference provider IDs and model names.
- Provider roles include at least chat, embeddings, vision, and remote
  transcription. Existing M2 remote provider settings must migrate or remain
  compatibly readable through the new registry.
- Remote providers remain opt-in. No private captured content is uploaded by
  default.
- Day grouping is configurable. The default day boundary is local
  midnight-to-midnight.
- Library settings include default projection, preview density, and day
  boundary.
- Library projections are days, recordings, sessions, and segments.
- Library list previews are private by default. They must not expose raw
  transcript text, OCR text, window titles, prompts, model responses, or private
  snippets unless the user explicitly chooses a denser preview mode.
- The day/recording detail default is the summary dashboard when indexed data is
  available.
- The playback studio remains the Evidence tab/surface.
- M3 Ask is search-only: hybrid search results with snippets, matched evidence
  type, score/source, and source jump actions. Generated cited Q&A is deferred.
- M3 follow-ups are internal suggestions only. No external todo, calendar, or
  reminder integration is implemented.
- Session taxonomy is user-editable and seeded with reasonable defaults such as
  work, entertainment, communication, and AFK.
- Unknown is a display and coverage state that can be shown or hidden. It is
  not a taxonomy entry.
- User edits to session labels, categories, colors, and notes persist as manual
  overrides and must survive regeneration.
- Model-generated structured records must be runtime-validated before
  persistence.
- M3 adds typed cross-platform window/app metadata interfaces.
- M3 implements and validates a Linux/KDE focused-window/open-window path when
  feasible. Unsupported platforms return typed unavailable states.
- M3 must not capture key values, typed text, clipboard content, or private
  window content outside already-approved screen/audio capture.
- Redaction invalidation must remove or invalidate overlapping day summaries,
  sessions, follow-ups, FTS rows, LanceDB vectors, search results, and derived
  source links.

## Success Criteria

The milestone is successful when:

- A completed recording/day can automatically continue from M2 evidence into
  M3 day-index stages.
- The user can configure OpenAI-compatible providers once and assign them to
  chat, embeddings, vision, and remote transcription roles.
- A local or remote OpenAI-compatible embedding endpoint can produce text
  embeddings for transcript, OCR, VLM, session, and follow-up/index text.
- LanceDB stores and retrieves local vectors with source provenance and
  redaction eligibility.
- SQLite FTS stores and retrieves lexical evidence records with source
  provenance and redaction eligibility.
- Hybrid search returns source-linked results over indexed day data.
- Optional LLM reranking can rerank top hybrid results when a chat provider is
  configured, and search still works without reranking.
- Day dashboard data is available through typed Tauri commands and rendered in
  Library detail with loading, empty, unavailable, error, indexed, and
  redacted/invalidated states.
- Library supports configurable projections and private default previews.
- Users can edit derived session labels, taxonomy categories, and notes, and
  regeneration preserves manual overrides.
- Internal follow-up suggestions are persisted with evidence links and review
  status.
- Redacting a span invalidates overlapping indexed data and removes it from
  search/dashboard results.
- Private transcript, OCR, window title, prompt, model response, embedding, and
  summary content is not logged by default.
- Required TypeScript and Rust quality gates pass.

## User Workflows and Test Matrix

| Workflow | State combinations | Automated test layer | Manual check | Privacy/logging risk |
| --- | --- | --- | --- | --- |
| Configure providers | no providers/provider saved/missing key/local endpoint failed/remote endpoint configured | Browser Mode, TS unit, Rust settings tests | Verify warnings for remote upload and local endpoint failures | API keys, endpoint URLs |
| Assign AI roles | chat only/embedding only/vision/transcription/shared provider/missing model | Browser Mode, TS unit, Rust validation tests | Verify role selectors and provider reuse | Provider role may imply private upload |
| Auto-index after AI processing | evidence complete/evidence partial/no transcript/no OCR/redacted recording/retry | Rust processing tests, Browser Mode | Verify notification and Library processing state | Private evidence enters embeddings/prompts |
| Day dashboard | empty day/indexing/indexed/failed/stale after redaction/manual overrides | Browser Mode, TS component tests | Desktop and narrow viewport review | Summaries and snippets are sensitive |
| Projection browsing | days/recordings/sessions/segments/private preview/dense preview | Browser Mode | Verify projection persistence and preview privacy | List previews can leak private content |
| Hybrid search | lexical match/vector match/both/no results/rerank unavailable/rerank failed | Rust search tests, Browser Mode | Verify source jump opens Evidence/playback | Query, results, snippets are sensitive |
| Session correction | generated/manual label/manual category/manual note/regeneration | Rust storage tests, Browser Mode | Verify overrides survive rebuild | Manual labels can reveal private context |
| Follow-up review | suggested/accepted/dismissed/invalidated by redaction/source jump | Rust storage tests, Browser Mode | Verify no external task/calendar mutation | Follow-up text is private |
| Redaction invalidation | transcript/OCR/VLM/session/follow-up/FTS/vector overlap | Rust invalidation tests, Browser Mode | Verify dashboard/search no longer expose removed span | Critical deletion risk |
| Window metadata | available/unavailable/permission denied/platform unsupported | Rust adapter tests, Browser Mode | Linux/KDE host check when safe | Window titles are sensitive |

## Scope

- Shared TypeScript contracts for provider registry, role bindings, day index
  settings, taxonomy, indexed sources, day summaries, derived sessions,
  follow-ups, hybrid search results, index coverage, and window metadata states.
- Rust settings and secret storage updates for reusable OpenAI-compatible
  provider definitions and role-specific model bindings.
- SQLite migrations for provider registry, role settings, taxonomy entries,
  day summaries, derived sessions, session overrides, follow-up suggestions,
  indexed text rows, FTS tables, vector metadata, index coverage, and window
  metadata events.
- LanceDB dependency, local app-data storage, table creation, text-vector
  writing, querying, and deletion/invalidation helpers.
- OpenAI-compatible provider adapters for `/v1/embeddings` and `/v1/responses`.
- Automatic downstream indexing stages after M2 evidence processing.
- Derived session and day summary generation using validated model output.
- Deterministic fallback coverage records when model generation is unavailable.
- Hybrid search APIs and UI.
- Library projection settings and dashboard-first detail UI.
- Internal follow-up suggestion review UI.
- Session taxonomy editing and manual session override UI.
- Cross-platform window metadata interfaces, Linux/KDE validation path where
  feasible, and unavailable states elsewhere.
- Privacy-safe logging and redaction invalidation coverage.

## Non-Goals

- Generated cited chat answers over the day index.
- Multi-day conversational memory.
- External todo, reminder, calendar, Slack, Linear, or email integrations.
- Bundling, downloading, or launching a local LLM runtime.
- Requiring working image embeddings for milestone closeout.
- Full visual similarity search over raw frames.
- Full cross-platform focused-window implementation on macOS and Windows.
- Speaker diarization.
- Perfect activity classification.
- Training or fine-tuning local classifiers from user corrections.
- Cloud sync, account sync, or cross-device Library settings.
- Public API for external apps to query the Daydream index.
- Redaction UI beyond the behavior needed to invalidate M3 derived data.

## Definitions and Ownership Rules

- Day index: the local searchable and summarized representation of a user's
  recorded working time.
- Day: a configurable local time window. The default is local
  midnight-to-midnight.
- Projection: a Library browse mode over the same underlying index. M3
  projections are days, recordings, sessions, and segments.
- Evidence record: source-derived text or metadata from transcript, OCR, VLM,
  activity, window metadata, or recording metadata.
- Indexed source: a normalized row that links search/digest/session output back
  to recording ID, source type, source record ID, and time range.
- Derived session: a time-bounded interpreted activity span with category,
  label, summary, confidence, and source provenance.
- Taxonomy entry: a user-visible category used for sessions and time
  allocation.
- Unknown coverage: indexed time with insufficient or conflicting evidence. It
  is display state, not taxonomy.
- Follow-up suggestion: an internal candidate todo, reminder, calendar item, or
  note extracted from evidence and linked back to source spans.
- Manual override: user-owned label/category/color/note edits that supersede
  generated session fields during display and survive regeneration.
- Provider definition: a reusable OpenAI-compatible endpoint configuration with
  label, base URL, encrypted key reference, and status metadata.
- Provider role binding: a setting that selects a provider and model for one
  role, such as chat, embeddings, vision, or remote transcription.
- React owns UI state, presentation, navigation, and Tauri command invocation.
- Rust owns SQLite, LanceDB, provider calls, processing/indexing jobs, native
  window metadata adapters, filesystem paths, and deletion/invalidation.
- SQLite owns durable metadata, FTS rows, provenance, settings, and user
  overrides.
- LanceDB owns vector payloads and vector-search metadata behind a Rust adapter.

## Architectural Invariants

- React must not query SQLite or LanceDB directly.
- React must not call OpenAI-compatible providers directly.
- All model calls must go through Rust-owned provider adapters.
- Provider adapters must runtime-validate responses before persistence.
- `/v1/responses` is the M3 generation compatibility floor.
- `/v1/embeddings` is the M3 embedding compatibility floor.
- Remote providers must be explicitly configured and visibly labeled before
  private data can be sent to them.
- Local providers are still external endpoints and must be labeled as such.
- Provider API keys must stay in encrypted secret storage and appear in frontend
  state only as secret references or redacted presence indicators.
- Every generated or indexed record must include source provenance and a
  redaction invalidation path.
- Search results must distinguish result source type: transcript, OCR, VLM
  observation, activity/window metadata, session, summary, or follow-up.
- Unknown/unavailable input states must not be silently converted into idle,
  AFK, or low-confidence activity.
- User overrides must be stored separately from generated fields.
- Regeneration may replace generated fields but must preserve manual overrides.
- No default logs may include transcript text, OCR text, VLM observations,
  window titles, prompts, model responses, embeddings, search snippets, or
  generated summaries.
- Day dashboard and Library preview UI must avoid exposing sensitive content in
  list views unless the user has selected a denser preview setting.

## Public API / Interface / Config Changes

M3 adds or updates shared payload concepts. Exact names may follow the repo's
existing conventions, but the behavior must remain equivalent.

Expected shared provider/settings concepts:

- `OpenAiCompatibleProvider`
- `OpenAiProviderRole`
- `OpenAiProviderRoleBinding`
- `ProviderRegistryState`
- `LibraryProjection`
- `LibraryPreviewDensity`
- `LibraryDayBoundarySettings`
- `DayIndexSettings`
- `SaveProviderInput`
- `DeleteProviderInput`
- `SaveProviderRoleBindingInput`
- `TestProviderInput`

Expected shared day-index concepts:

- `DayIndexScope`
- `DayIndexStatus`
- `DayIndexCoverage`
- `LibraryDaySummary`
- `LibraryDayDashboard`
- `LibraryProjectionItem`
- `DerivedSession`
- `DerivedSessionSource`
- `SessionTaxonomyEntry`
- `SessionManualOverride`
- `FollowUpSuggestion`
- `FollowUpSuggestionStatus`
- `HybridSearchInput`
- `HybridSearchResult`
- `HybridSearchSource`
- `WindowMetadataState`
- `WindowFocusEvent`
- `OpenWindowSnapshot`

Expected Tauri commands:

- `listOpenAiProviders() -> ProviderRegistryState`
- `saveOpenAiProvider(input: SaveProviderInput) -> ProviderRegistryState`
- `deleteOpenAiProvider(input: DeleteProviderInput) -> ProviderRegistryState`
- `testOpenAiProvider(input: TestProviderInput) -> ProviderTestResult`
- `saveOpenAiProviderRoleBinding(input: SaveProviderRoleBindingInput) -> ProviderRegistryState`
- `getDayIndexSettings() -> DayIndexSettings`
- `saveDayIndexSettings(input: SaveDayIndexSettingsInput) -> DayIndexSettings`
- `listLibraryProjectionItems(input: ListLibraryProjectionItemsInput) -> LibraryProjectionItemList`
- `getLibraryDayDashboard(input: GetLibraryDayDashboardInput) -> LibraryDayDashboard`
- `listDerivedSessions(input: ListDerivedSessionsInput) -> DerivedSessionList`
- `saveDerivedSessionOverride(input: SaveDerivedSessionOverrideInput) -> DerivedSession`
- `listSessionTaxonomy() -> SessionTaxonomyList`
- `saveSessionTaxonomyEntry(input: SaveSessionTaxonomyEntryInput) -> SessionTaxonomyList`
- `deleteSessionTaxonomyEntry(input: DeleteSessionTaxonomyEntryInput) -> SessionTaxonomyList`
- `listFollowUpSuggestions(input: ListFollowUpSuggestionsInput) -> FollowUpSuggestionList`
- `updateFollowUpSuggestion(input: UpdateFollowUpSuggestionInput) -> FollowUpSuggestion`
- `searchDayIndex(input: HybridSearchInput) -> HybridSearchResultList`
- `rebuildDayIndex(input: RebuildDayIndexInput) -> ProcessingJob`
- `getWindowMetadataStatus() -> WindowMetadataState`

Expected setting keys:

- `ai.providers.<providerId>.label`
- `ai.providers.<providerId>.baseUrl`
- `ai.providers.<providerId>.apiKeyRef`
- `ai.roles.chat.providerId`
- `ai.roles.chat.model`
- `ai.roles.embeddings.providerId`
- `ai.roles.embeddings.model`
- `ai.roles.embeddings.dimensions`
- `ai.roles.vision.providerId`
- `ai.roles.vision.model`
- `ai.roles.transcription.providerId`
- `ai.roles.transcription.model`
- `library.defaultProjection`
- `library.previewDensity`
- `library.dayBoundary.mode`
- `library.dayBoundary.startLocalTime`
- `library.dayBoundary.timezone`
- `library.showUnknownCoverage`

Migration compatibility:

- Existing M2 remote transcription and vision settings must remain readable.
- If M3 migrates existing remote settings into provider registry rows, the
  migration must preserve encrypted secret references and must not expose
  plaintext keys.
- If M3 keeps legacy settings as compatibility aliases during migration, the
  milestone closeout must document which settings remain legacy and which
  provider registry roles own new writes.

## Native Permission Requirements

- Window/app metadata capture is privacy-sensitive native metadata.
- M3 adds typed cross-platform interfaces for focused-window identity, focused
  window title, and open-window snapshots.
- Linux/KDE is the required implementation/validation target when feasible.
- macOS and Windows may return typed unavailable states in M3.
- If Linux/KDE capture requires native permissions, accessibility privileges,
  compositor-specific APIs, or session limitations, the UI and diagnostics must
  show a repairable unavailable or permission state.
- Window titles and open-window lists must not be logged by default.
- Window metadata must be disabled or unavailable without faking data when the
  platform cannot safely provide it.
- Clipboard content is out of scope. Clipboard event timing is also out of
  scope for M3 unless a later milestone explicitly enables it.

## Privacy and Deletion Contract

- M3 indexes highly sensitive private data: transcripts, OCR, VLM observations,
  window metadata, generated summaries, embeddings, and follow-up suggestions.
- No remote provider is used unless the user explicitly configures and selects
  it for a role.
- The UI must label when embeddings, generation, reranking, VLM, or remote
  transcription may upload private data to an external endpoint.
- Default logs must not include private indexed content.
- Provider errors must use private-content-safe messages. They may include
  endpoint host, status class, and error code, but not prompts, transcript text,
  OCR text, model responses, or embeddings.
- Redaction invalidation must remove or invalidate overlapping:
  - indexed source rows
  - SQLite FTS rows
  - LanceDB vector rows
  - day summaries
  - derived sessions and session source links
  - follow-up suggestions
  - search-result cache rows if any are added
  - prompt/input audit rows if any are added
- Redacted content must not remain visible through Library projections,
  dashboard aggregates, search snippets, follow-up suggestions, session labels,
  or source links.
- If an entire generated session is invalidated by redaction, user overrides for
  that session must not resurrect private generated content. The UI may show an
  invalidated manual note only if it does not include removed source text.

## Storage Format

M3 adds explicit storage for reusable providers, day-index settings, indexed
sources, generated artifacts, search metadata, and user overrides. Exact column
names may follow Rust conventions, but the schema must preserve these
relationships.

Expected SQLite tables:

- `openai_providers`
  - provider ID, label, base URL, API key secret ref, created/updated
    timestamps, last validation status, last validation error code
- `openai_provider_role_bindings`
  - role, provider ID, model name, optional dimensions for embeddings,
    created/updated timestamps
- `day_index_settings`
  - default projection, preview density, day boundary mode/time/timezone,
    unknown coverage visibility
- `session_taxonomy_entries`
  - category ID, label, color, sort order, enabled state, seeded/manual flag,
    created/updated timestamps
- `indexed_sources`
  - source ID, recording ID, optional day ID, source type, source record ID,
    start/end offsets, text hash, provider/model provenance, redaction state,
    created/updated timestamps
- `indexed_source_fts`
  - FTS-backed text for lexical search, joined to `indexed_sources`
- `vector_index_entries`
  - source ID, LanceDB table name, vector row ID, modality, embedding provider,
    model, dimensions, content hash, redaction state, created/updated
    timestamps
- `day_summaries`
  - day ID, day start/end, generated overview, aggregate coverage JSON,
    provider/model/prompt version, status, created/updated timestamps
- `derived_sessions`
  - session ID, day ID, start/end, generated category ID, generated label,
    generated summary, confidence, status, provider/model/prompt version,
    created/updated timestamps
- `derived_session_sources`
  - session ID, indexed source ID, support kind, confidence/weight
- `derived_session_overrides`
  - session ID, manual category ID, manual label, manual note, manual color,
    updated timestamp
- `follow_up_suggestions`
  - suggestion ID, day ID, optional session ID, type, generated text, status,
    due/candidate time if available, provider/model/prompt version,
    created/updated timestamps
- `follow_up_sources`
  - suggestion ID, indexed source ID, support kind, confidence/weight
- `window_focus_events`
  - recording ID, start/end offset or timestamp, app identity, window title,
    platform metadata JSON, availability status, redaction state
- `open_window_snapshots`
  - recording ID, timestamp, window inventory JSON, availability status,
    redaction state

LanceDB storage:

- LanceDB data lives under the app data directory, separate from SQLite.
- M3 creates at least one text-embedding table with vector, source ID, source
  kind, recording ID, day ID, start/end offsets, provider/model, content hash,
  and redaction state.
- M3 may create an image-embedding table schema, but working writes are not
  required for closeout.
- LanceDB table versioning or compaction details must be recorded in the
  scratchpad if implementation decisions affect deletion semantics.

Migration stop boundary:

- Schema edits and embedded migrations happen in the same gate.
- Migration tests must run with:
  `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- Migrations must not depend on React.
- Migrations must not expose plaintext API keys or private indexed content in
  logs or test snapshots.

## Processing Contract

- M3 indexing is downstream of M2 evidence processing.
- If transcript/OCR/VLM/activity evidence is missing, M3 writes honest coverage
  states rather than inventing complete sessions.
- Indexing creates durable chunks or stage records before provider calls.
- Each indexable source keeps recording ID, source type, source record ID, time
  range, redaction state, provider/model provenance where applicable, and
  content hash.
- Text source construction must be deterministic so retries can replace or
  skip unchanged rows.
- Embedding writes must be idempotent by source ID, content hash, provider,
  model, and dimensions.
- Failed embedding/generation chunks must be retryable without duplicating
  indexed rows.
- Partial failures leave available lower-layer data visible. For example, FTS
  search can work when embeddings fail.
- Sessionization consumes indexed evidence and activity/window metadata. It may
  use an LLM when chat provider settings are valid.
- LLM-produced sessions, summaries, and follow-ups must conform to Daydream
  runtime validation schemas before persistence.
- If LLM output is invalid, M3 stores a failed chunk with a safe error and does
  not persist unvalidated structured claims.
- Deterministic fallback coverage records are allowed when generation is
  unavailable, but they must be labeled as unavailable/ungenerated in UI.
- Search result snippets must come from indexed text rows and must respect
  redaction state.
- Optional LLM reranking receives only top result snippets and provenance
  needed for ranking. It must not receive raw full-day evidence unless a later
  milestone explicitly expands that behavior.

## Search Contract

- Hybrid search runs within an explicit scope: day, recording, session, or full
  Library projection where supported.
- Lexical search uses SQLite FTS over indexed text rows.
- Vector search uses LanceDB over text embeddings.
- Result merging must deduplicate by source ID and preserve match type:
  lexical, vector, or both.
- Optional reranking may reorder only the merged top-k result set.
- Search works without a chat provider by returning non-reranked hybrid
  results.
- Search works without embeddings by returning FTS-only results and a visible
  degraded index state.
- Search works without FTS by returning vector-only results and a visible
  degraded index state.
- Search results include source type, title/label, snippet, timestamp range,
  score or rank metadata, and a jump target into the Evidence surface.
- Search results must not include redacted sources.

## UI Contract

- Library overview gains projection controls for days, recordings, sessions,
  and segments.
- Library settings expose default projection, preview density, day boundary,
  and unknown coverage visibility.
- Private preview is the default density and must avoid raw private snippets.
- Dashboard detail is the default for day and indexed recording/session views.
- The dashboard shows:
  - processing/index coverage
  - active/idle/AFK/unknown time
  - time by taxonomy category
  - derived sessions with confidence and source jumps
  - follow-up suggestions with review status
  - redaction/stale index state
- Evidence tab routes to or embeds the existing playback studio.
- Search tab shows hybrid search results with source jumps.
- Session editing lets users override category, label, color, and note.
- Follow-up review lets users mark suggestions as pending, accepted, dismissed,
  or invalidated internally only.
- Disabled or unavailable states must explain the blocker near the relevant
  action: missing provider, failed index, no evidence, permission unavailable,
  or redacted/stale source.

## Gate A: Provider Registry and Role Settings

- [ ] Add shared provider registry and role binding types.
- [ ] Add Rust settings/storage support for reusable OpenAI-compatible
  providers and role-specific model bindings.
- [ ] Migrate or compatibly bridge existing M2 remote transcription and vision
  settings into provider registry semantics.
- [ ] Add encrypted secret handling for provider API keys without exposing
  plaintext keys in frontend state, diagnostics, notifications, or logs.
- [ ] Add provider test/status commands for `/v1/responses` and
  `/v1/embeddings` capability checks.
- [ ] Add Settings UI for provider definitions and role assignment.
- [ ] Add tests for provider CRUD, role validation, legacy settings
  compatibility, secret redaction, and remote-upload warning states.

Exit Criteria:

- Users can define an OpenAI-compatible provider once and assign it to chat,
  embeddings, vision, and remote transcription roles.
- Existing M2 remote provider settings remain usable after migration.

## Gate B: Day Index Storage and LanceDB Adapter

- [ ] Add shared day-index, taxonomy, session, follow-up, hybrid search, and
  coverage types.
- [ ] Add embedded SQLite migrations for M3 provider, settings, FTS, indexed
  source, session, follow-up, summary, vector metadata, and window metadata
  tables.
- [ ] Add LanceDB as a Rust dependency after checking official package/crate
  metadata.
- [ ] Implement a Rust LanceDB adapter for local app-data path creation, table
  initialization, text-vector upsert, vector search, and source deletion.
- [ ] Add storage helpers for indexed sources, FTS rows, vector metadata,
  taxonomy entries, derived sessions, overrides, summaries, and follow-ups.
- [ ] Add migration and adapter tests for fresh database, upgraded database,
  idempotent writes, and redaction-state filtering.

Exit Criteria:

- SQLite and LanceDB can store, query, and remove indexed text/vector sources
  with source provenance.

## Gate C: Indexing Pipeline Integration

- [ ] Add downstream durable processing stages/chunks for indexing,
  embeddings, sessionization, summary generation, follow-up extraction, and
  search-index writes.
- [ ] Automatically enqueue M3 indexing after M2 evidence reaches eligible
  completion.
- [ ] Add manual rebuild/retry commands for day, recording, and session scopes.
- [ ] Build deterministic indexed text sources from transcript, OCR, VLM,
  activity, window metadata, and recording metadata.
- [ ] Implement OpenAI-compatible `/v1/embeddings` calls for text sources.
- [ ] Persist FTS rows and LanceDB vectors idempotently by source hash,
  provider, model, and dimensions.
- [ ] Add processing tests for partial evidence, retry, cancellation,
  unchanged-source skipping, and private-content-safe errors.

Exit Criteria:

- A completed M2 recording can automatically produce FTS and vector index
  records, and failures can be retried without duplication.

## Gate D: Derived Sessions, Taxonomy, and Summaries

- [ ] Seed default taxonomy entries for work, entertainment, communication, and
  AFK.
- [ ] Add taxonomy edit, enable/disable, color, label, and sort-order flows.
- [ ] Implement source-linked derived session generation from indexed evidence.
- [ ] Use `/v1/responses` for structured session/day summary/follow-up output
  when a chat provider is configured.
- [ ] Runtime-validate all generated structured output before persistence.
- [ ] Persist generated sessions separately from manual overrides.
- [ ] Preserve manual session/category/label/note overrides across
  regeneration.
- [ ] Add deterministic fallback coverage when generation is unavailable.
- [ ] Add tests for taxonomy defaults, manual overrides, invalid model output,
  regeneration, and unavailable provider states.

Exit Criteria:

- Indexed days can show source-linked generated sessions and summaries, and
  user corrections survive rebuilds.

## Gate E: Hybrid Search

- [ ] Implement SQLite FTS search over indexed source rows.
- [ ] Implement LanceDB vector search over text embeddings.
- [ ] Merge and deduplicate lexical/vector results with source provenance.
- [ ] Add optional `/v1/responses` reranking over top merged results when a
  chat provider is configured.
- [ ] Add `searchDayIndex` command and frontend client wrapper.
- [ ] Add search UI for day/recording/session scopes with loading, empty,
  degraded, error, reranked, and non-reranked states.
- [ ] Add source-jump behavior into the Evidence/playback surface.
- [ ] Add tests for lexical-only, vector-only, hybrid, rerank unavailable,
  rerank failed, redacted source exclusion, and source jumps.

Exit Criteria:

- The user can search an indexed day and jump from each result to source
  evidence without generated chat answers.

## Gate F: Library Projections and Dashboard UI

- [ ] Add Library settings for default projection, preview density, day
  boundary, and unknown coverage visibility.
- [ ] Add Library projection item APIs for days, recordings, sessions, and
  segments.
- [ ] Update Library overview with projection controls and private default
  previews.
- [ ] Add day dashboard detail with overview metrics, coverage state, time
  allocation, session list, follow-up summary, and processing/redaction status.
- [ ] Keep route and screen files thin by moving dashboard/projection UI into
  feature components.
- [ ] Add Evidence tab entry point to the existing playback studio.
- [ ] Add responsive Browser Mode tests for projection switching, dashboard
  states, private preview behavior, source jumps, and narrow viewport layout.

Exit Criteria:

- Library can be used as a day-index dashboard while preserving the current
  recording/playback Evidence path.

## Gate G: Follow-Up Suggestions

- [ ] Persist internal follow-up suggestions with source provenance and status.
- [ ] Render follow-up suggestions in the dashboard and detail review surfaces.
- [ ] Support pending, accepted, dismissed, and invalidated statuses.
- [ ] Add source jumps from each follow-up to Evidence.
- [ ] Ensure accepted/dismissed follow-ups remain internal only and do not
  create external tasks, reminders, or calendar entries.
- [ ] Add tests for follow-up extraction, status updates, source jumps,
  provider unavailable states, and redaction invalidation.

Exit Criteria:

- The user can review and classify internal follow-up suggestions without any
  external integration side effects.

## Gate H: Window Metadata Interfaces and Linux Validation

- [ ] Add shared window metadata status, focus event, and open-window snapshot
  types.
- [ ] Add Rust native adapter interfaces for focused app/window and open-window
  inventory.
- [ ] Implement typed unavailable states for unsupported platforms.
- [ ] Implement and validate a Linux/KDE path when feasible.
- [ ] Persist window metadata events with source timing and redaction state.
- [ ] Include window metadata in indexed source construction only when
  available and allowed.
- [ ] Add diagnostics/UI states for unavailable, permission denied, and active
  window metadata capture.
- [ ] Add tests for adapter state mapping, storage, indexing, and private
  logging.

Exit Criteria:

- Window metadata is either captured through a validated Linux path or
  represented honestly as unavailable without fake activity data.

## Gate I: Privacy, Redaction Invalidation, and Final Validation

- [ ] Implement invalidation helpers for M3 day summaries, sessions,
  follow-ups, indexed sources, FTS rows, LanceDB vectors, and vector metadata.
- [ ] Wire redaction creation/deletion flows to mark M3 derived data stale,
  removed, or rebuild-eligible according to overlap.
- [ ] Add privacy-safe logging assertions for provider calls, prompts, outputs,
  search snippets, window metadata, and embeddings.
- [ ] Verify remote provider warning copy in Settings, processing, and
  dashboard/search states.
- [ ] Run focused Rust tests for storage, processing, provider, search, and
  redaction invalidation.
- [ ] Run focused Browser Mode tests for dashboard, projections, search,
  session overrides, follow-ups, and redaction states.
- [ ] Run repository quality gates:
  - `COREPACK_HOME=/tmp/corepack corepack yarn lint:fix`
  - `COREPACK_HOME=/tmp/corepack corepack yarn lint`
  - `COREPACK_HOME=/tmp/corepack corepack yarn typecheck`
  - `COREPACK_HOME=/tmp/corepack corepack yarn test`
  - `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml`
- [ ] Perform desktop and narrow viewport manual checks for Library dashboard,
  projection switching, search, follow-up review, and Evidence navigation.

Exit Criteria:

- M3 indexed data respects redactions and private-content logging constraints,
  and the milestone passes required quality gates.

## Testing and Validation

Required automated coverage:

- Shared type tests for provider registry, day-index payloads, search results,
  taxonomy, session overrides, and follow-ups.
- Rust migration tests for fresh and upgraded databases.
- Rust storage tests for indexed sources, FTS rows, taxonomy, sessions,
  overrides, follow-ups, provider registry, and redaction state.
- Rust LanceDB adapter tests for table creation, upsert, query, and source
  deletion where the local test environment permits.
- Rust provider tests for `/v1/responses`, `/v1/embeddings`, invalid JSON,
  missing model, failed endpoint, missing key, and safe error mapping.
- Rust processing tests for automatic downstream indexing, retry,
  cancellation, partial evidence, and idempotent rebuild.
- Rust redaction tests for source overlap invalidation across SQLite and
  LanceDB.
- Browser Mode tests for provider settings, role assignment, Library
  projections, dashboard, search, session correction, follow-up review, source
  jumps, and unavailable states.

Required manual validation:

- Desktop viewport review of Library projections, day dashboard, search, and
  Evidence navigation.
- Narrow viewport review of the same surfaces.
- Linux/KDE window metadata validation when host permissions and session
  conditions allow it.
- Remote provider warning review for chat, embeddings, vision, and remote
  transcription roles.

Quality gates:

- `COREPACK_HOME=/tmp/corepack corepack yarn lint:fix`
- `COREPACK_HOME=/tmp/corepack corepack yarn lint`
- `COREPACK_HOME=/tmp/corepack corepack yarn typecheck`
- `COREPACK_HOME=/tmp/corepack corepack yarn test`
- `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml`

## Dependencies and Risks

- LanceDB Rust SDK behavior, binary dependencies, and storage semantics must be
  verified from official docs and crate metadata before implementation.
- `/v1/responses` is a stricter compatibility floor than many local
  OpenAI-compatible servers support. Provider status checks must make this
  failure clear.
- M3 embeddings can upload private evidence to remote providers. Settings and
  processing UI must label this clearly.
- Hybrid search quality will depend on embedding model choice, dimensions,
  chunking, and text-source construction.
- Day dashboard UX can overstate model certainty. Unknown coverage and
  confidence must remain visible.
- Redaction invalidation now spans SQLite and LanceDB. Partial invalidation is
  a serious privacy bug.
- Window metadata capture may be compositor/session-specific on Linux and may
  not be portable. Unsupported states are acceptable; fake data is not.
- Provider registry migration can break M2 remote transcription/VLM if legacy
  settings are not bridged carefully.
- Large all-day evidence can be expensive to embed and summarize. M3 must use
  durable chunks and resumable jobs.

## Assumptions

- M2 evidence processing is the prerequisite input layer for M3.
- M3 may render unavailable or partial states when M2 evidence is missing or
  incomplete.
- Local app data is the right storage root for LanceDB.
- Text embeddings are enough to make the first day-index search useful.
- Image embeddings need table/adapter planning in M3 but not a working provider
  path for closeout.
- Users are willing to configure an external local or remote OpenAI-compatible
  endpoint for LLM and embedding behavior in M3.
- Generated cited Q&A is valuable, but search-only Ask is the correct M3
  boundary.
- Follow-up suggestions are useful without external task/calendar integration.

## Expected Deliverables

- `docs/milestones/m3-day-index-mvp.md`
- `docs/milestones/m3-day-index-mvp-scratchpad.md`
- Provider registry and role settings.
- LanceDB-backed text vector index.
- SQLite-backed FTS and day-index metadata.
- Downstream day-index processing stages.
- Derived sessions with editable taxonomy and manual overrides.
- Internal follow-up suggestions.
- Hybrid search with source jumps.
- Dashboard-first Library detail.
- Projection-aware Library overview and settings.
- Window metadata interfaces with Linux validation or honest unavailable
  states.
- Redaction invalidation for all M3 derived/indexed data.
