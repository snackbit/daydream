# Milestone M3: Day Index MVP Scratchpad

## Purpose

Live notes for M3 planning, implementation findings, validation logs, and
follow-up candidates. The milestone contract is
`docs/milestones/m3-day-index-mvp.md`.

## Planning Inputs

- `docs/wip-library-day-index-design.md` establishes the product direction:
  Library should become a private day index with playback as evidence.
- M2 owns the evidence layer: transcription, activity, OCR, VLM observations,
  jobs, provider settings, and playback integration.
- M2.1 owns the Recording Studio timeline/editor surface.
- M3 should not duplicate M2 evidence processing or M2.1 playback work. It
  should consume them.

## Locked Planning Decisions

- M3 scope is Day Index MVP.
- LanceDB is the vector store for M3.
- SQLite FTS remains part of hybrid search.
- Search is FTS plus vector plus optional model reranking.
- Text embeddings are required.
- Image embeddings are optional for closeout but need adapter/table boundaries.
- M3 uses OpenAI-compatible `/v1/responses` for generation.
- M3 uses OpenAI-compatible `/v1/embeddings` for embeddings.
- Local LLM/embedding support means user-configured external local endpoints.
- Daydream does not bundle a local LLM runtime in M3.
- Provider settings become a provider registry. Role-specific settings
  reference provider IDs and model names.
- Roles include chat, embeddings, vision, and remote transcription.
- Indexing is automatic after M2 evidence processing reaches eligible
  completion.
- The user-facing processing concept remains AI processing, with downstream M3
  stages.
- M3 Ask is search-only. Generated cited Q&A is deferred.
- Follow-ups are internal suggestions only.
- Day boundary is configurable and defaults to local calendar day.
- Library gets full settings for default projection, preview density, and day
  boundary.
- Session taxonomy is user-editable and seeded with defaults.
- Unknown is a coverage/display state, not a taxonomy entry.
- Session/user corrections persist as overrides across regeneration.
- Window metadata gets typed cross-platform interfaces.
- Linux/KDE focused-window/open-window implementation is desired when feasible;
  unsupported platforms return unavailable states.

## Repo Evidence

- Existing shared AI types already include `digest` and `embedding` processing
  job kinds, but M2 lists daily digest, task/todo extraction, embeddings, vector
  search, and chat as non-goals.
- Existing shared AI types include transcript, activity, OCR region, and VLM
  observation concepts that M3 should consume.
- Existing Library is recording-centric with grid/list overview and
  `/library/$recordingId` playback studio route.
- Existing Recording Studio has player, track timeline, inspector, redaction
  range concepts, and derived AI lane strips.
- Existing settings have transcription and vision provider settings; M3 must
  avoid breaking those while moving to provider registry semantics.
- Current Rust dependencies do not include LanceDB.

## External Reference Notes

- LanceDB docs describe OSS as an embedded local library with Python,
  TypeScript, and Rust SDKs, local path storage, vector search, full-text search,
  hybrid search, and metadata filtering capabilities.
- LanceDB API docs list the Rust SDK as officially supported.
- OpenAI embeddings accept model, input string or array, optional dimensions,
  and float/base64 encoding format, and return vectors.
- OpenAI Responses supports structured output with JSON schema, but M3 must
  still runtime-validate all model output before persistence.
- Many local OpenAI-compatible servers may support `/v1/chat/completions` more
  commonly than `/v1/responses`; M3 intentionally chooses `/v1/responses` as
  the compatibility floor.

## Risk Watchlist

- `/v1/responses` may reduce local endpoint compatibility. Provider tests and
  clear unavailable states matter.
- LanceDB deletion semantics must be checked before relying on physical removal
  for privacy. If physical delete/compaction is nuanced, milestone contract may
  need a stronger invalidation-plus-compaction rule.
- Provider registry migration could disrupt M2 remote transcription/VLM paths.
- Embedding full-day transcripts/OCR can be expensive. Chunk sizing and
  resumability must be validated during implementation.
- Search snippets can leak private content if preview density rules are reused
  incorrectly.
- Unknown coverage must stay visible enough that dashboard summaries do not
  overclaim.
- Linux window metadata may depend on compositor/session APIs and may be
  unavailable on Wayland without extra permissions.

## Follow-Up Candidates

- M4 cited chat/Q&A over the indexed day.
- Multi-day search and memory.
- External todo/calendar/reminder integrations.
- Local bundled LLM runtime management.
- Working image embeddings and visual similarity search.
- Cross-platform window metadata implementation beyond Linux/KDE.
- Learning from user taxonomy/session corrections.
- Meeting-note export and share workflows.

## Validation Log

- Pending.

## Gate Notes

### Gate A: Provider Registry and Role Settings

- Pending.

### Gate B: Day Index Storage and LanceDB Adapter

- Pending.

### Gate C: Indexing Pipeline Integration

- Pending.

### Gate D: Derived Sessions, Taxonomy, and Summaries

- Pending.

### Gate E: Hybrid Search

- Pending.

### Gate F: Library Projections and Dashboard UI

- Pending.

### Gate G: Follow-Up Suggestions

- Pending.

### Gate H: Window Metadata Interfaces and Linux Validation

- Pending.

### Gate I: Privacy, Redaction Invalidation, and Final Validation

- Pending.
