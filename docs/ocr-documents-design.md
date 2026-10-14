# Full-frame OCR documents

Approved implementation contract for the OCR work after main `51ad981`, aligned
with the implemented storage, provider and UI boundaries. JavaScript gates,
native verification and independent UI acceptance passed. This
is ordinary feature work, not a new milestone. The final UI verification record is
[OCR review](ocr-review.md).

## User workflow

Recognized text is internal retrieval input. Day review, search results and Studio
do not display OCR text, revision diffs or raw word details, including in Debug.
Screen matches open the recorded source moment; selecting a frame retains its
timestamp, source, recognition status and processing recovery controls. Audio
transcripts remain visible in playback. Preserve recognized words and their
provenance internally rather than deleting uncertain retrieval input.

Users choose local Tesseract (default) or configured VLM OCR. VLM OCR reuses the
existing Vision endpoint, model and backend-held key reference, but is independent
of the visual-observations enabled toggle. Explain that enabling VLM OCR can send
private full-frame images to the configured endpoint. Provider errors show Error
and Retry; do not silently fall back to Tesseract.

Sampling cost is visible beside the setting: at 0.2 FPS the pipeline samples 12
frames per minute. Exact-image reuse can reduce recognition/provider calls below
that count; enabling observation optimizations does not lower the OCR sample rate.

Existing raw OCR rows remain readable through a best-effort geometric reconstruction.
Label those results as legacy reconstruction: they do not prove document continuity
or temporal history. Opening legacy data must not automatically process frames or
make remote requests.

## Recognition and temporal identity

Adaptive v1 is an explicitly selectable alternative to the full sampling contract
below. New installations default to it; upgrades retain Full until the user changes
preferences or chooses **Use fast local processing**. It checks candidates at the
slower of the configured cadence and one frame every five seconds, recognizes
material tile changes, and takes a full checkpoint every 30 seconds. Exact pixel
reuse retains occurrences. Other omissions publish `gap` occurrences and never
substitute earlier text. Privacy cuts, gaps, configuration changes and restart
reset comparison history conservatively. Existing job snapshots decode as Full
and keep their original cache fingerprints. Scoped retries keep that policy;
explicit reprocessing captures the new policy.

- Recognize the entire original-size decoded frame at every frame selected by the
  existing user-controlled recognition sampling policy. Do not introduce a new
  five-second cadence or turn recording FPS into an implicit OCR schedule.
- Tesseract keeps TSV block/paragraph/line/word hierarchy and geometry for reading
  order. The document boundary retains the full recognized text and uncertain words.
- Literal VLM OCR has a separately validated output section from descriptive observations.
  Send an original-size full PNG with high detail; no 1600px resize, OCR-history
  context, interpretation hints or previous-frame text. Validate model output
  before persistence. Protocol `literal-ocr-v1` accepts only ordered text blocks and
  rejects added geometry/confidence fields. Its boxes and confidence stay null;
  original image dimensions are native facts, not model estimates.
  New general-processing jobs may share a request with material-change observations
  when source, frame, sampling and provider/model configuration match. The OCR
  section still reads only the current full frame; up to three eligible earlier
  same-source frames provide observation context. Invalid sections retry independently.
  Legacy and OCR-only jobs retain their existing request protocol.
- Cache only exact decoded-image identity within a compatible engine/request
  configuration. Reuse avoids repeated recognition requests; it does not skip
  sampled occurrences or fabricate timestamp provenance.
- After recognition, group identical text into temporal revisions using whitespace
  normalization only. Preserve case, numbers and locations/geometry. Every
  occurrence keeps its recording/frame/time source. Content identity and occurrence
  identity remain separate, so the UI can explain where a revision was observed.
- Redaction boundaries split eligible history. Cache reuse and text grouping must
  not reconnect hidden spans, revive invalidated data or imply continuity through
  a removed occurrence.

Timeline frames and text revisions are distinct views. The timeline must retain
each actual sampled occurrence, including separate timestamps that reuse identical
content. Legacy reconstructed frame points remain selectable. Processing progress
is a job signal, not an observed frame and must not become a stretched frame marker.
The normal timeline uses bounded OCR occurrence metadata rather than restoring a
whole-recording raw-word payload. The new metadata command is documented below
only after its native/shared contract is published.

This approved full-frame recognition policy supersedes M2's changed-region OCR
policy for this feature. Frame-delta activity and descriptive visual observations
keep their separate existing policies; full-frame OCR does not authorize broader
summary, audio or capture changes.

## Durable boundaries and compatibility

Rust/SQLite owns documents, ordered blocks/content, sampled occurrences and temporal
history. React uses typed commands to query selected-frame recognition metadata,
not raw SQLite or a whole-recording word payload. Shared types are in
`packages/shared/src/ai/ocr.ts`; Rust DTOs are in `ai/ocr_types.rs`.

| Command | Input and result |
| --- | --- |
| `getOcrDocument` | Recording/source track/timestamp, optionally a document ID. Returns a nullable document and the latest applicable OCR job, including failures before any document exists. |
| `getOcrHistory` | Recording, optional source track, limit and `{timestampMs, id}` cursor. Defaults to 100 entries, clamps to 1–200; orders observed start descending then ID descending. Returns entries and a nullable next cursor. |
| `reprocessRecordingOcr` | Recording ID. Queues current-engine OCR-only work for a completed, eligible recording without active OCR work; returns its processing job. |

SQLite schema 20 adds `ocr_document_contents`, `ocr_frame_occurrences` and
`processing_job_ocr_configs` to schema 19. Successful content and every sampled
occurrence remain distinct: successful/empty, failed, gap and reused samples keep
their own provenance. Content reuse is indexed by recording/source, exact decoded
pixels, dimensions and configuration identity. Configuration fingerprints include
engine/model/request version and local Tesseract binary/trained-data identity using
streamed SHA-256 hashes. Persistence commits check current eligibility and attempt
ownership atomically; hidden old unique-key matches cannot restore earlier output.
Paid successful recognition remains reusable after pause/resume or retry when its
source is still eligible and the exact image/configuration matches. Cache support
is separate from display eligibility: current-attempt/status filtering hides old
results, while prior committed success can avoid charging the provider again.
Redaction/protection applies to both paths. Retry clears raw/frame-state output but
retains eligible content support; deletion removes occurrences and orphan content.

Legacy migration reconstructs geometric reading order from existing raw rows locally.
Its provenance is `reconstructed`, engine is `legacy`, and unknown dimensions are
null at the public boundary. A sampled point uses a one-millisecond internal interval
for lookup; that is not evidence of continuity. Legacy entries never coalesce across
time or trigger automatic OCR/provider calls. The old raw rows remain available.

Processing jobs pin OCR settings and provider identity when created. Existing queued
jobs remain local Tesseract jobs. Retry uses the original job configuration; an
explicit OCR reprocess uses the newly selected engine and must not rerun unrelated
audio processing. New settings do not retroactively change an in-flight provider.

Eligibility checks, attempt leases, request drainage, redaction invalidation,
deletion and private cache cleanup remain authoritative at each side-effect
boundary. A current redaction must exclude both local work and remote upload, and
deleting a recording must remove its OCR documents/occurrences/cache artifacts.
No raw OCR text, image, prompt, response or key enters ordinary logs.

## Frame extraction compatibility

Full-frame OCR keeps original image dimensions and lossless PNG output. Extraction
uses the user's existing frame-recognition FPS or sync-to-recording setting, with
two decoder threads and one PNG encoder thread. It uses FFmpeg `-fps_mode passthrough`
to retain filtered timestamps rather than the removed `-vsync` option. The same
compatibility correction applies to the existing observation extractor; it does not
change observation sampling or resize policy. [Official FFmpeg options](https://ffmpeg.org/ffmpeg.html#Main-options)
define passthrough timestamp behavior.

The current Linux managed-development binary resolved by `ffmpeg-static@5.3.0` is
FFmpeg `7.0.2-static`; its `-h full` lists `-fps_mode` and marks `-vsync` deprecated.
The host FFmpeg 9 extraction failure was observed before this correction. This
feature adds no FFmpeg package/version upgrade; other packaged platform binaries
still need their normal distribution validation.

## Ownership and acceptance

| Owner | Responsibility and evidence |
| --- | --- |
| Native storage/contracts | Explicit migration, selected-frame/history queries, ordered document/occurrence mapping, legacy reconstruction, deletion and redaction eligibility tests. |
| Processing | Full-frame runners, literal VLM validation, exact-image reuse, whitespace-only revisions, pinned jobs, retry/reprocess and failure/recovery tests. |
| Frontend | Source-linked recognition metadata, no raw OCR presentation, loading/error/retry and keyboard/browser workflows. |
| Settings | Engine selection, independent OCR/observations controls, remote disclosure, dirty/save and unavailable-provider states. |
| Independent review | Actual desktop/narrow workflows in both themes, source navigation and failure recovery, with synthetic media and visible assertions. |

Final gates run once after the coherent source batch settles: coordinated native
full tests, formatting and strict all-target Clippy with two build jobs; root
JavaScript autofix/lint/typecheck/tests and desktop production build; independent
UI grade at least 4/5 for the changed workflow. Exact commands and runtime limits:
[validation map](agent-validation.md). Do not mark implementation or cleanup complete
until the owning evidence and integrated gates pass.

Focused evidence currently includes 33 OCR-filtered tests (including 13 storage
cases) and AI 98/settings 11 ownership checks. These sets overlap and must not be
summed as unique coverage. The migration was observed failing because the new table
was missing, and actual extraction was observed failing on removed `-vsync` before
the fixes. Other new runner features were implemented before compilation; do not
claim every new case had an observed red-first run.

Final main-thread verification passed autofix, lint, all workspace typechecks and
348 tests: desktop 338, shared 9 and FFmpeg 1. The production build passed with its
existing eager-chunk advisory (947.76 kB / 277.18 kB gzip). Independent Inspector
review passed 4/5 in both themes in three passes; Settings passed 4/5 in one.
The native suite passed 381 unit tests before the finishing permit/cache/index
changes; the final OCR namespace passed 40 tests covering those changes, and the
two permit tests passed separately. These overlapping sets are not summed.
Doc-only retry passed (0 examples), formatting passed, and strict all-target
Clippy passed after its two diagnostics were corrected.

Timeline navigation uses every eligible occurrence from the lightweight index,
including reconstructed legacy samples, rather than processing progress or
coalesced text history. Filled cells extend to the next observation in the same
source/segment; the final cell extends to the segment boundary. Physical clicks
inside cells select their own observation timestamp. Main-thread styled Browser
tests and public-app checks passed in both themes and at desktop/narrow widths;
actual evidence bounds remain separate from navigation geometry.

Real private recordings, hosted provider reliability, native permissions and model
OCR accuracy are separate manual risks. Existing GTK/GLib advisory and dependency
maintenance limits are unchanged by this feature; no dependency upgrade is implied.
