# Recall behavior and architecture

Days provides factual capture coverage, cited interpretations, corrections,
suggestions and local tasks. Search retrieves eligible OCR and both audio
transcript tracks. Opening a result resolves its current source and privacy
eligibility before navigating to Studio.

## Availability and consent

Text search and factual browsing remain available without generation or a local
embedding model. Local models require an explicit download. Embeddings default
to managed multilingual E5 on CPU; Auto and Vulkan have separate runtime
requirements described in [embedding runtime](embedding-runtime.md).

Remote embeddings and Day generation each require an enabled configuration and
consent. Vision consent enables neither. Feedback remains local. Tasks have
open, done and dismissed states; they never execute external actions.

Generation waits for a five-minute quiet period and runs at most once per
fifteen minutes during recording. Explicit regeneration remains available.
One active request and one latest pending request bound generation work.

## Evidence and storage

Rust owns Recall services and SQLite persistence. Typed Tauri commands and client
decoders form the frontend boundary. Metadata-only change events invalidate reads.

Evidence retains recording, segment, source, time range, content revision and
input provenance. New segments have durable timeline bases. Ambiguous legacy
timing is excluded from timed Recall until repaired; playback offsets are not
guessed. Days follow local calendar boundaries, including DST.

Retrieval combines FTS5 with normalized embeddings behind an adapter. Long
documents split into bounded, source-linked passages. Source eligibility applies
before ranking and display. OCR text remains internal retrieval input; screen
hits open their recorded source moment.

Generation validates structured output and evidence references in Rust. Lineage
includes all consumed inputs, including uncited prompt material. Privacy,
deletion, reprocessing and configuration changes invalidate late results.
Affected task and interpretation text stays withheld while user edits and
decisions remain stored. Retry and restart reuse valid completed work.

## Verification boundaries

Storage and service tests cover migration, timestamps, redaction, deletion,
configuration races, retry and recovery. Browser workflows cover navigation,
search, source links, review and edits. Model tests use synthetic passages.
Real-provider quality and device behavior require separate checks.

See [processing privacy](processing-privacy-boundaries.md),
[OCR behavior](ocr-documents-design.md) and
[test workflows](user-story-validation.md).
