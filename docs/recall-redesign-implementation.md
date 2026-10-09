# Recall redesign implementation

The user approved the full desktop redesign and real recall features on 2026-10-06.
Use `/tmp/tmp.9ea4LVcvPz/DESIGN.md` and its app screenshots as the visual reference.
This is implementation, not a mock prototype. Root coordinates all agents and checks.

## Using the implemented workspace

Open Library → Days to review the latest unreviewed past day, or choose Today.
Recordings remains available for the existing recording list and Studio controls.
Day contains factual coverage, cited interpretations, session corrections, local
feedback, suggestions and kept tasks. Search can use text-only or hybrid retrieval;
opening a hit resolves its current eligibility and exact Studio source without
starting playback.

In Settings → AI/LLM → Recall, download/select a managed embedding model or
explicitly enable an embedding API. Generation is independently disabled until
its endpoint/model/authentication are configured and enabled. Its tests use
synthetic input. Local feedback is never included in remote prompts. Local models
do not download automatically. Missing models/providers leave factual browsing and
available text search usable with honest readiness/coverage messages.

Schema 25 adds recall data and stable segment bases. Historical settings restore
local embedding defaults without enabling remote generation. Existing media stays
compatible; ambiguous legacy timing is withheld from timed recall rather than
guessed. See [embedding runtime](embedding-runtime.md) for build/model pins and
actual CPU/Vulkan proof, and [verification record](refactoring-tasks.md) for final
gates and remaining operator checks.

## Locked product decisions

- Library opens on Days, prioritizing the most recent unreviewed past day; Today
  and Recordings remain nearby. Keep real privacy-aware thumbnails.
- Add semantic plus lexical search of eligible OCR and both transcript tracks.
  Existing VLM descriptions remain in Studio, not authoritative recall input.
- Add grounded Day summaries/sessions/suggestions, local ratings/corrections,
  editable local tasks (open/done/dismissed), and reviewed input fingerprints.
- Generate automatically after a five-minute quiet period; while recording, cap
  each Day at one generation per fifteen minutes. One active plus one latest pending.
- Embeddings offer app-managed local CPU/Auto/Vulkan and OpenAI-compatible API.
  Local default: multilingual E5 Small Q8_0; optional BGE Small English v1.5 Q8_0.
  Pinned checksums/licenses and actual model/tokenization/pooling tests are required.
- Generation uses a separately enabled OpenAI-compatible text profile. Remote
  embeddings and generation need their own explicit consent; Vision consent is not
  inherited. API-key and anonymous authentication are supported.
- New installations default to light; saved light/dark choices are preserved.
  Apply spacious prototype composition throughout Capture and Studio too.
- Feedback is local. Tasks never cause email/calendar/external actions. Withhold
  affected source-derived task/interpretation text after privacy changes; keep edits
  and decisions durably without using them to regenerate hidden source content.

## Design system

One semantic Tailwind v4 theme and existing shadcn owners. Inter remains bundled.
Light surfaces: #fdfcfc / #f5f3f1 / #ebe8e4; text #000000 / #44403b / #6d675f.
Dark surfaces: #191715 / #24211e / #302b26; text #fdfcfc / #ded8d1 / #b7aea4.
Pill buttons/tabs/badges; 20px panels, 24px workspace/dialogs, 8px controls.
Page heading 36px/300 (32px narrow); body 14px, controls 13px, metadata 12px.
Padding 24px panels, 32px workspace, 20px narrow. Standard 4px Tailwind spacing
must not be overwritten by the reference's numeric spacing aliases.
Topbar 64px, persistent recording strip >=56px. Gutters 40/28/20px. Standard pages
share 1280px max width; Capture/Studio fill available width. Sidebar 216px wide,
72px rail at medium widths, official shadcn Sheet below 768px. Add official Switch.
Colors in features use tokens; geometry is an allowed dynamic-style exception.
Functional recording/error/privacy state and annotation colors remain distinguishable.
No imported prototype CSS, fake data/state, external fonts or duplicate primitives.

## Native boundaries and data

- Focused `recall` native service with shared typed domain contracts/client decoders.
- Canonical evidence preserves recording/segment/source, timeline range, trusted
  absolute time, content revision and input provenance. OCR eligibility must reuse
  current occurrence/protection/redaction/attempt/supersession semantics.
- Durable segment bases for new recordings; conservative legacy backfill. Missing
  predecessors must not shift trusted offsets. Ambiguous legacy evidence is excluded
  from timed recall until repaired; existing playback is not guessed or rewritten.
- Day uses actual local-calendar timezone boundaries, including DST/midnight.
- FTS5 and normalized vectors behind an adapter. Score all eligible scoped vectors
  in bounded batches, fuse rankings deterministically, and never silently cap scans.
- Indexing and cross-recording Day jobs have real recall ownership, not fake
  recording IDs. Integrate their progress/actions into Processing.
- Validate generated JSON and supplied evidence IDs/ranges in Rust. Track ALL
  consumed inputs, including uncited material. Reject late results after privacy,
  deletion, reprocessing or configuration changes. Never log private text/bodies.
- Reuse successful work/checkpoints on restart/retry. Configuration/auth/schema
  errors require recovery; transient calls have bounded persisted retries.
- An isolated packaged native embedding worker avoids a second ggml copy linked
  into Whisper. Bounded private stdio; cancellation/crash handling; no Python runtime.

## Integration protocol

The contracts agent is authoritative for shared/native DTO names and recall
provider settings. Publish the first complete contract early and message root.
Proposed boundary names: listRecallDays, getRecallDay, searchRecallEvidence,
resolveRecallEvidence, regenerateRecallDay, setRecallFeedback,
setRecallSessionCorrection, acceptRecallSuggestion, updateRecallTask,
setRecallDayReviewed, getRecallStatus, testRecallProvider and controlRecallJob.
Native services accept `&AppStorage` plus validated inputs where practical so real
SQL/behavior can be tested with synthetic fixtures. Commands remain thin.
Metadata-only `recall-state-changed` invalidates client reads. Source links extend
the existing Studio route and revalidate eligibility before seek/inspection.
Profiles live under `AppSettings.ai.recall`; separate embedding/generation keys
extend the existing atomic credential save. Local embeddings and disabled
generation are defaults; readiness never blocks unrelated settings/capture.

## Ownership and checks

Workers are not alone: preserve others' edits. No nested delegation, commits or
user-facing status messages; send findings and completion to root. Use apply_patch
for semantic changes, only owned-file formatting, and follow scoped AGENTS/skills.
Root schedules ALL Cargo, CMake model builds, browser servers and final gates.
Do not launch expensive checks without a granted slot. Small unit/lint/type reads
can be coordinated separately. Share two native build jobs. Use pinned Corepack Yarn.

Required proofs: real native privacy/migration/retry/timestamp failures; browser
workflows for existing controls plus Day/Search/review; both themes at 1440/1024/
768/390 and 320px control reachability; keyboard/contrast/reduced motion/zoom;
CPU English/German model inference and backend handling; 10k/100k/1m vector
benchmarks. 100k x384 warm search target <1s and <=32MB incremental memory, with
prompt cancellation. Limits must produce explicit failure/incomplete state.
Final immutable install, JS/native gates and production build after edits settle.
No ambient real screen/microphone capture; record actual device/package checks
performed or pending. Models/runtime/GPU quality are not presumed validated.
