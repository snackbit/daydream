# Independent OCR review

Baseline: `51ad981`, main. Review date: 2026-10-05. OCR Inspector and Settings meet grade 4/5 in dark/light mode at desktop/narrow widths. Inspector required three coherent passes; Settings required one. The later timeline regressions are resolved: per-observation metadata restores separate samples, and filled navigation cells preserve the original click surface. After the user's request to stop subagents, the main agent completed the final cell-width regression tests, public-app checks and aggregate gates.

The reviewer owns this report and temporary browser fixtures/screenshots only. Product implementation belongs to the native storage, processing, frontend and Settings owners. Heavy native checks and aggregate gates remain coordinated by the main agent. All review fixtures are synthetic; private recordings, OCR text and user screenshots are not copied or logged.

## Approved invariants

- Tesseract is the default; Vision OCR is an explicit alternative using the existing Vision endpoint/model/key. Screen observations have a separate enable switch and must not consume/delete a secret that OCR still needs.
- Either engine recognizes the full original image at every frame selected by the existing recognition cadence. There is no second fixed OCR cadence, crop-only policy or 1600px Vision OCR resize.
- Vision OCR uses a literal-text JSON request with only the current full image, validates its result, exposes truncation/provider failures and offers Retry. It never silently falls back to Tesseract or fabricates bounding boxes/confidence.
- Exact unchanged pixels may reuse successful content within recording/source/configuration scope, while every sampled timestamp receives an occurrence. Whitespace-only equivalent text merges into one adjacent revision with supporting sample times. Case, punctuation, names and numbers remain meaningful changes.
- Failure, empty text, gaps, source/segment/configuration changes and redactions split revisions. Legacy reconstruction is labelled best effort and never invents continuous spans or enables remote processing.
- History is paginated in the backend; selecting a document fetches its blocks/words separately. Normal Inspector payloads must not load all historical raw words. Low-confidence Tesseract text remains visible and marked; unknown Vision confidence/geometry remains null.
- Redaction immediately hides selected text, search and diff, closes history spans, invalidates cached reuse and blocks stale responses. Native authorization and eligibility are rechecked after asynchronous work.

## Review method and quality bar

Review runtime contracts and privacy invariants first, then actual user workflows. Browser fixtures mock only native/IPC/media/clipboard boundaries; the real React UI, router, controllers and shadcn components run normally. Implementation batches must mount and typecheck before grading to avoid incomplete API migrations.

Fresh surface grades: 1 unusable, 2 weak, 3 unfinished, 4 professional with bounded residual issues, 5 exceptional. At most four coherent passes per new OCR surface. Both dark/light and desktop/narrow layouts must meet the same bar. Precision/restraint 25%, information hierarchy/density 30%, workflow 15%, interaction/accessibility 20%, responsive/state 10%. A functional raw-word or oversized-card presentation cannot earn 4.

## Risk and evidence matrix

| Workflow / invariant | Failure to protect against | Review proof | Status |
| --- | --- | --- | --- |
| Read a 500-word recognized document | Raw word rows, clipping, unreadable paragraphs or quiet omission of uncertain words | Text/read-dialog screenshots; real bounded wheel scroll to last paragraph | Passed both themes, 1440/620 |
| Find, Copy and Jump | Wrong match, copied debug payload, lost source timestamp or keyboard trap | Exact literal Copy, highlighted Find, real player offset, one Escape/focus restoration | Passed both themes |
| Compare revisions | Digit/name/chat changes suppressed by fuzzy normalization; camera pixel changes duplicate identical text | Exact digit/name Changes; native whitespace-only grouping and decoded-pixel cache tests | Passed UI and bounded native review |
| History and selected document | Entire word corpus fetched, broken cursor or cross-source ID lookup | Lazy history IPC limit 20 and cursor; normal raw-region query disabled; source-scoped Rust query | Passed |
| Vision OCR configuration and errors | Accidental upload, lost shared key, fallback or duplicate cadence | Settings save/disable/engine-switch; actual source-lane pinned failure Retry | Passed both themes |
| Original-image literal OCR | Crop/resize/context summarization, invalid/truncated provider JSON persisted | Full-image high-detail request, text-only schema and truncation/invented-box regressions | Source and target tests verified; model accuracy not benchmarked |
| Cache and revision boundaries | Reuse after cut/config change or false continuity across failures/gaps/legacy | Scoped reuse, privacy-before-window query, actual support times, storage scenarios and resume delta | Source review passed; final resume test/aggregate result coordinated separately |
| Cut while reading/searching | Stale document/diff/history remains visible or a delayed reply repopulates it | Public ruler Redact with delayed IPC; durable open-dialog/history privacy tests | Passed both themes |
| Legacy and queued jobs | Remote reprocessing without explicit action or retry changes old engine | Labelled point reconstruction, nullable dimensions, pinned legacy Tesseract, exact-job Retry | Passed source/UI proof |

## Historical ownership checkpoints

Native storage owner reports content scope recording/source/pixel hash/decoded dimensions/config fingerprint, successful content only, and distinct timestamped occurrences with status/cache/segment provenance. Revision query adjacency is previous sample-end equals next timestamp, with same segment/source/configuration and whitespace-only literal equivalence. Legacy entries do not coalesce. Cache lookup requires a currently visible supporting occurrence; redaction boundaries break runs even when hidden rows are filtered. This is a design checkpoint, not verified implementation.

Settings controls are planned as Text recognition engine, Run OCR on retained frames and independent Generate screen observations, sharing Base URL/API key/Vision model. The request estimate must identify the OCR-only ceiling; observations may generate additional traffic under their own policy. Source/configuration errors must remain explicit.

Cross-integration check raised early: current Studio marker loading eagerly consumes recording-wide `ocrRegions` from `getRecordingVisualObservations`. Adding new document/history endpoints alone does not make normal load lightweight. Frontend and storage owners were asked to coordinate frame/occurrence summaries for OCR-only markers and retain raw regions only for explicit Details/debug/legacy paths. This is a required integration check, not a finding against an unfinished patch.

## Limits

Browser review cannot certify real OCR model accuracy, provider billing, operating-system permissions, packaged WebKit behavior or physical storage deletion. Those require coordinated native tests and, where needed, manual application checks. No remote provider request will be made from the browser review; synthetic IPC results stand in for that boundary. Final handoff will separate verified behavior from these limits and include actual commands/results and canonical inspected design references.

## Independent review passes and implementation findings

The public Studio route rendered synthetic 500-word documents at 1440×960 and 620×960 without page errors or horizontal overflow. The first pass uses natural paragraphs, repeated meeting text, a low-confidence retained word, and a second revision changing `12500.00` to `12501.00` and a synthetic name. Text/Changes/Details remain compact and neutral; readable text replaces word-coordinate rows. Vision confidence is visibly unknown, and reconstructed legacy evidence is labelled. Evidence is under `/tmp/daydream-ocr-pass1-light`, `/tmp/daydream-ocr-pass1-dark` and `/tmp/daydream-ocr-settings-pass1-dark`.

- **Historical Inspector grade 3, pass 1:** reading, Find, exact Copy, source Jump, lazy history and pagination passed. The reading dialog initially focused an uncertain-word tooltip; its unsolicited overlay obscured the first paragraph and two Escape presses did not close the dialog. That keyboard defect prevented acceptance and was corrected in pass 2.
- **Historical Inspector grade 3, pass 2:** reading content received focus, no unsolicited tooltip appeared, one Escape closed the dialog, and the uncertainty action remained keyboard accessible. Both themes passed 500-word Find/Copy/Jump, exact pinned-job Retry from a Video source with zero recognized frames, lazy paginated Changes, and privacy recovery. Screenshot inspection then exposed a separate sizing defect: at 620px the modal was 816px tall and the ScrollArea root 726px, but its viewport grew to 980px and text spilled beyond the bottom border. Acceptance was withheld until a definite bounded viewport and real wheel-scroll proof. Evidence: `/tmp/daydream-ocr-pass2-{dark,light}` and `/tmp/daydream-ocr-pass2-scroll-diagnostic/evidence.json`. This defect was corrected in pass 3.
- **Inspector accepted grade 4, pass 3:** a definite height within the existing 85vh cap gives the ScrollArea a bounded viewport. At 1440px the viewport is 726px tall with 780px of text; at 620px it is 726px with 980px of text. Real wheel scrolling reaches the final paragraph inside the modal, and one Escape closes it. Both-theme screenshots have no text spilling outside the modal, page errors or horizontal overflow. Find, exact Copy and source Jump still pass. Evidence: `/tmp/daydream-ocr-pass3-{dark,light}/evidence.json`. Passes 1 and 2 above are historical defects, now corrected; no fourth pass is needed.
- **Settings accepted grade 4 in both themes, pass 1:** desktop/narrow layouts show the engine choice, independent observations, one endpoint/model/key and explicit configuration blocker. At 0.2 FPS the disclosed OCR ceiling is 12 sampled frames/minute, before exact-image reuse, with separate observation traffic and no invented price. Actual enable/save/disable/save and engine-switch interactions preserve the opaque backend key reference; no secret deletion occurs. Evidence: `/tmp/daydream-ocr-settings-checked-{dark,light}/evidence.json` and `/tmp/daydream-ocr-settings-pass1-{dark,light-corrected}` screenshots. The observation-only optimization wording is corrected; no further visual round is needed.
- **Privacy workflow passes on the public route in both themes:** while Find/Changes displayed recognized text, a second document response was held at the IPC boundary. A real ruler selection and Redact action removed it; resolving the late request did not restore text, search or diff. Evidence: `ocr-cut-late-document` in `/tmp/daydream-ocr-settings-pass1-dark/evidence.json` and `/tmp/daydream-ocr-settings-pass1-light-corrected/evidence.json`. Component Browser Mode separately covers privacy updates closing an already-open reading dialog.
- **Vision geometry corrected:** the initial protocol accepted arbitrary in-bounds model boxes. Bounds are not evidence that coordinates are measured. The final literal prompt and strict text-only response schema now reject geometry/extra fields, including in-bounds invented boxes; persisted Vision block geometry and confidence are null. Tesseract keeps measured words and confidence.
- **Pre-frame failure integration corrected:** runtime/configuration/FFmpeg errors can happen before any occurrence. Native now supplies source/time/privacy-scoped `selection.job`. Both-theme public-route tests with zero recognized-frame markers select the Video source, show its failed-job error, and retry that exact pinned job ID. No fictional frame or Tesseract fallback is introduced. Eligible successful text remains readable when an unrelated audio portion of a mixed parent job fails; the frontend durable Browser test covers that priority.
- **Settings policy wording corrected:** the retained similarity-cache description referred to skipping similar regions between full-frame passes. The control is now labelled Optimize screen observations and explains that OCR processes each selected full frame, reusing only exact image matches. It no longer implies approximate/crop OCR.

Harness-only failures from an outdated tab/checkbox accessible name, a reverted draft with disabled Save, and the newly required `selection.job` field are excluded from product grades. They were diagnosed and corrected before repeating affected cases.

## Native review checkpoint

Bounded independent source inspection confirms the following; this is code/test evidence, not a claim that the reviewer made real model calls:

- Full-resolution FFmpeg extraction follows the existing effective frame cadence. Decoded RGBA pixels, original dimensions and configuration/runtime identity feed SHA-256 cache keys. Tesseract runtime and trained-data bytes are stream-hashed, avoiding reuse after assets change at the same path.
- Tesseract preserves words below 0.5 confidence and measured hierarchy/geometry. Vision sends one current original-size PNG with high detail, no prior OCR/summary context, and accepts literal text-only blocks. It rejects truncated responses and invented geometry without local fallback.
- Enqueued configuration is pinned; absent legacy job configuration resolves to local Tesseract. Explicit OCR-only reprocessing takes the current configuration under the recording mutation lease. Eligibility is checked before and after asynchronous recognition and again during atomic persistence.
- Occurrence IDs and raw-word links remain timestamp-specific when content is reused. Fresh recognition cannot accidentally recover hidden old content through a unique-key conflict; explicit reuse requires currently eligible successful support. Source/configuration/dimensions scope the cache.
- Privacy and current-attempt filtering happen before revision windows. Only whitespace equivalence coalesces native adjacent samples; empty/failure/gap/segment/configuration/legacy boundaries remain distinct. Ranges end at the last actual supporting timestamp, not an invented continuous observation end. History returns metadata/text and a bounded cursor page; selected document alone reads block JSON. `includeOcrRegions:false` skips the old raw-region SQL entirely.
- Legacy migration reconstructs one frame at a time, labels approximate reading order, retains only point evidence and reports unknown image dimensions. Deleting processing chunks deletes occurrences and orphan content. Source/time/privacy-scoped job status explains a pre-frame failure without fabricating a frame.

Coordinated native evidence read: `/tmp/daydream-ocr-final-target.log` reports **33 passed, 0 failed**, including 13 storage/query/cache/legacy scenarios and full-resolution FFmpeg cadence. Processing owner additionally reports fresh-binary AI 98 and Settings 11 checks; final native/aggregate gates remain the coordinator's responsibility.

Final cache/resume delta was reviewed separately. The shared eligibility CTE still applies current source redactions and protection to both paths. Display adds current-attempt/status checks; cache reuse accepts only previously committed successful native content with exact recording/source/dimensions/pixels/configuration, even after pause/retry. Retry clears raw/frame-state output while retaining this cache support; explicit deletion still removes documents and orphan content. Atomic writes retain `chunk_active` and revalidate the specific cache content, so old workers cannot commit and old attempt text cannot appear as current evidence.

The new regressions first failed against the prior behavior: committed cache was unavailable after retry, and a synthetic inference adapter ran 3 times instead of the expected 2. Actual red proof is `/tmp/daydream-ocr-resumption-red.log`. Restored final source is verified; `/tmp/daydream-ocr-resumption-green.log` and the final aggregate gate are owned by the native/coordinator workflow. These are synthetic call-count checks, not real provider billing measurements. Historical target and full-suite counts must not be summed as unique coverage.

Professional text/source hierarchy is calibrated against previously inspected [Rive selected-object properties](https://mobbin.com/screens/877d36f2-2265-498f-b13f-33b0c9626ae5), [Figma selected-layer properties](https://mobbin.com/screens/fb73f09d-1f19-4257-8c65-855f85dc116e) and [Descript source-linked editor](https://mobbin.com/screens/53b0795d-a62e-4d6a-8252-e9df70e6d574). These references inform compact source identity, focused content and quiet secondary metadata; they are not OCR correctness evidence. The inspected Mobbin calls returned no AI usage notice.

## Final UI matrix and evidence

| Surface | Dark | Light | Fresh passes used | Visible acceptance |
| --- | --- | --- | --- | --- |
| OCR Inspector: Text, Changes, Details and reading view | 4/5 | 4/5 | 3 of 4 | Literal paragraphs, retained uncertainty, exact Find/Copy/Jump, source identity, lightweight lazy history, exact changes, bounded long-text reading and keyboard dismissal |
| OCR/Vision Settings | 4/5 | 4/5 | 1 of 4 | Engine choice, independent observations, shared endpoint/key, honest original-image/cadence/billing disclosure, explicit blocker/no fallback and saved-secret retention |

Grade 4 means professional with bounded residuals, not exceptional. The compact 11px Inspector reading style remains dense; at narrow width, long text is most comfortable in the expanded reading view. No critical interaction, layout or privacy regression remains in the reviewed UI. There is no reason to spend another cosmetic round.

The independent harness runs the public app at `http://127.0.0.1:1431` with only native/media/clipboard boundaries replaced. Commands use `node /tmp/daydream-ui-review.mjs` with `REVIEW_THEME`, `REVIEW_SCOPE`, `REVIEW_INTERACTIONS` and `REVIEW_INTERACTION_SCOPE` to select coherent cases; narrow interactions use `REVIEW_DEFAULT_INTERACTION_WIDTH=620`. The final pass 3 selects `ocr-text,ocr-reading,ocr-changes` plus `ocr-read-find-copy-jump,ocr-reading-scroll-bounds`. Screenshots and machine-readable evidence are retained in `/tmp/daydream-ocr-pass3-dark` and `/tmp/daydream-ocr-pass3-light`.

Additional reviewed proof:

- `/tmp/daydream-ocr-pass2-{dark,light}`: zero-frame source selection, actual scoped failure, exact pinned `retryProcessingJob` ID, history limit/cursor and real player destination. No `reprocessRecordingOcr` replaces a pinned job retry.
- `/tmp/daydream-ocr-settings-pass1-{dark,light-corrected}`: real ruler Redact with a delayed document response; late resolution cannot revive the text/search/diff. Vision confidence unknown and non-debug Details have no raw-word control.
- `/tmp/daydream-ocr-settings-checked-{dark,light}`: observations enable/save/disable/save and engine switching retain the backend secret reference without secret deletion.
- `/tmp/daydream-ocr-states-narrow-light`: document-load failure/Retry recovery and keyboard opt-in/close of Debug raw details. `/tmp/daydream-ocr-final-states-dark` additionally renders Vision, reconstructed legacy and gap states at both widths with no page errors or horizontal overflow.
- Durable Browser tests cover public Studio mounting, 500-word reading, uncertainty, source-linked Changes, legacy evidence, latest-provider failure, mixed-parent failure priority, processing events, exact pinned retry and immediate privacy removal of an open reading dialog/history. Frontend final scoped handoff reports 80 Browser and 7 unit checks; Settings handoff reports 19 Browser checks. Coordinator owns the final aggregate count and gate result; overlapping targeted counts are not summed as unique coverage.

Representative final screenshots: `/tmp/daydream-ocr-pass3-dark/ocr-text-1440.png`, `/tmp/daydream-ocr-pass3-light/ocr-reading-1440.png`, `/tmp/daydream-ocr-pass3-light/ocr-reading-620.png`. They contain synthetic text only.

## Reopened timeline regression: individual OCR samples

The user reported one large OCR marker despite multiple timestamped legacy entries in Changes. Independent public-route reproduction uses three synthetic legacy documents at 0, 5 and 10 seconds, with one coarse progress state at 0 and no raw OCR regions. The renderer produces only `Visual frame 0.0s to 30.0s, no VLM, OCR complete`; the assertion for three individually selectable samples fails **1 ≠ 3**. Evidence: `/tmp/daydream-ocr-timeline-legacy-red/evidence.json` and `ocr-index-legacy-coarse-1440.png`. No private screenshot text is copied into this report.

The cause is the metadata path, not whitespace/text revision grouping. Normal `includeOcrRegions:false` correctly removes raw words but supplies no replacement per-occurrence timestamp index. Studio builds frames only from processing progress states, raw OCR regions or VLM observations. One coarse state therefore becomes one frame extended to the segment end; empty progress states lose OCR markers entirely. Changes reads durable documents through a separate endpoint and can still contain all samples.

The repair uses authoritative lightweight metadata for every actual sampled occurrence, independent of cached content/revision grouping and progress availability. It preserves real source/segment/timestamp/reference identities and privacy/current-attempt/latest-job rules without inferring a fixed five-second cadence or restoring the recording-wide raw-word payload.

The user then clarified that restoring tiny point markers is insufficient: the original filled navigation cells must remain easy to see and click. Navigation hit geometry is separate from observation evidence. A cell starts at its actual sample timestamp, ends at the next actual sample from the same source, and the last cell ends at its containing segment boundary. Clicking anywhere in that cell selects the cell's own timestamp/document, not the pointer's timestamp. Actual `sampleEndMs`, legacy point evidence, history support and privacy eligibility stay unchanged. The final synthetic check therefore requires filled cells 0–5, 5–10 and 10–30 seconds, with physical midpoint clicks at 2.5, 7.5 and 25 selecting samples 0, 5 and 10. Merely counting three tiny markers will not earn functional acceptance.

## Main-thread completion and final gates

- The styled Browser regression physically clicks inside navigation cells at 2.5,
  7.5 and 25 seconds and verifies selection of observations 0, 5 and 10. It passed
  at 1440/620 pixels in both themes. Legacy/coarse-progress, pagination and late
  redaction-index regressions also passed: 7 focused Browser tests in total.
- The public app passed filled-cell/source-document checks in both themes, with
  desktop/narrow screenshots and zero runtime errors. Empty progress still shows
  all legacy observations; three native observations can share one Changes revision
  without merging timeline cells. Evidence: `/tmp/daydream-ocr-main-filled-light`
  and `/tmp/daydream-ocr-main-filled-dark`. An initial harness failure treated the
  name `empty-progress` as empty OCR text; the fixture was corrected, not production.
- Final `corepack yarn check` passed lint, all typechecks and 348 tests (desktop 338,
  shared 9, FFmpeg 1). Production build and `git diff --check` passed. Logs:
  `/tmp/daydream-ocr-main-check.log` and `/tmp/daydream-ocr-main-build.log`.
- Native full unit run passed 381 tests before finishing changes. Final OCR checks
  passed 40 tests, including current cache-resumption and timeline cases; the two
  permit tests passed separately. Doc-only retry (0 examples), formatting and strict
  all-target Clippy passed. Logs: `/tmp/daydream-ocr-native-timeline-green.log`,
  `/tmp/daydream-ocr-native-doc.log`, `/tmp/daydream-ocr-native-clippy.log`.
  These overlapping test sets are not summed as unique coverage.
- Hosted-model accuracy, real WebKit playback and OS capture remain untested manual
  boundaries. The existing bundle-size and GLib advisory limitations remain.

## On-demand reading refinement

The Inspector now shows a compact word/uncertainty summary and Read OCR text/Copy
actions. Selecting a frame does not render its recognized text. Full text and Find
are available in the explicit reading dialog; Copy works from either surface, and
closing the dialog removes the text from the Inspector again.

The default-hidden assertion was observed failing before the change. The updated
OCR/Studio Browser workflows pass 80 tests, including search, exact full-text copy,
source navigation, revision selection and privacy clearing. The final root check
passes 348 tests, lint and all typechecks; production build and diff checks pass.
Public-app light/dark screenshots at 1440/620 show the compact default with zero
runtime errors: `/tmp/daydream-ocr-on-demand-light` and
`/tmp/daydream-ocr-on-demand-dark`.
