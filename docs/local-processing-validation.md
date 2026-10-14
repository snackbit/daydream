# Local visual processing and search recovery

The slow backlog used remote Vision OCR on 5120×1440 screens. Local operation
metadata showed a median 369 seconds per 30-second OCR section and 52 seconds
per observation section. Completion events continued, so the available evidence
supports very slow processing rather than a confirmed deadlock. An old failed
CPU index attempt also prevented admission after switching to Vulkan. Settings
did not persist processing timing or resource mode.

## Result and activation

Processing now offers **Use fast local processing** for finished recordings.
It drains visual owners, saves the local Tesseract/adaptive/Fast preset and admits
one visual replacement job. Completed audio, activity and waveform results stay
intact. Unfinished nonvisual sections are restored, including paused intent.
Recovery snapshots survive an interrupted operation and can be retried through
the same action. No private recordings or app settings were modified during
implementation or validation; activation requires running the updated app and
using this action on each affected recording.

Adaptive v1 checks at most one candidate every five seconds, recognizes material
changes and checkpoints every 30 seconds. Exact-image reuse retains each sample's
own timestamps; omitted nonidentical samples become gaps. Observations have a
separate 30-second minimum spacing and 60-second heartbeat. Brief or subtle content
can be missed. Existing jobs retain their full policy until explicit replacement.

Local OCR and observations have separate bounded worker capacity. Shared frame
extraction retains at most four entries and 256 MiB of PNG files; each entry retains
one decoded image up to 32 MiB. Adaptive history is capped at four entries and
128 MiB. Identical encoded frames reuse decoded pixels. Subprocesses have output,
time and cancellation bounds. Indexing submits 16 documents per embedding call,
holds sorted recording privacy leases through publication and revalidates inputs.
Failed index admission matches the execution configuration as well as vector space.
Frame counts, current index failures and measured per-lane estimates are visible.
Estimates are withheld when samples are insufficient or work appears stalled.

Schema v40 adds recovery snapshots, an index-admission index and an explicit full
sampling preference for upgraded installations. Existing command names remain;
the processing control adds `optimize-visual`. New IPC fields are optional for
older frontend fixtures. No dependencies, remote model selections or public
capture permissions changed.

## Scenario evidence

| Workflow or invariant | Evidence |
| --- | --- |
| Backend change after failed index attempt | Native regression first reproduced blocked admission; matching current work can now run while pause/cancel intent remains. |
| Real embedding batches | Native tests verify batched calls, invalid-vector rejection without partial publication, cancellation and privacy invalidation. |
| Adaptive sampling | Decoded synthetic screens verify material changes, noise, checkpoints, configuration/privacy changes and noncontiguous history. |
| Exact image reuse | FFmpeg-generated frames share decoded image ownership while preserving independent timestamps; OCR cache and source provenance regressions remain. |
| Recovery preservation | Real SQLite tests keep completed speech searchable and reload interrupted recovery snapshots to restore nonvisual work and pause intent. |
| Persistence and migration | Native settings tests preserve unrelated settings; migrations distinguish fresh defaults, upgraded full policy and explicit adaptive preferences. |
| Processing UI | Browser workflows cover recovery success/failure, live-recording guard, progress counts, indexing guidance and backward-compatible payload decoding. |
| Sampling control | Settings browser workflow saves both policies, reopens the sheet and retains processing timing and resource mode. |

Browser tests substitute typed IPC boundaries. Native tests use isolated SQLite
databases, generated media and controlled provider boundaries. No real captured
text, prompts or recordings enter fixtures. Rust branch instrumentation is not
available in the existing validation environment. Live Vision quality, real
recording OCR accuracy and Vulkan throughput require separate observation.

The focused processing suite passes 132 tests with 97.11% measured branch coverage
(337/347); every included processing owner exceeds 90%. The changed processing
settings owner has 100% measured branches (2/2), exercised through 11 settings
browser workflows. These reports do not imply coverage of unchanged settings
callbacks. Native formatting and strict all-target Clippy pass. The full native
gate passes 714 library tests plus three resource-staging tests; five opt-in tests
are ignored by that default gate. The throughput test was run explicitly.
The final JS gate passes lint, all typechecks, 1,061 desktop tests, nine shared
tests and one FFmpeg test. Its broad browser run emitted existing ResizeObserver
diagnostics; the focused processing and settings runs passed independently.

The actual Tauri bootstrap and inference phases pass against fresh isolated app
data. They cover shared inference/retry, scoped Processing presentation, summary
failure/retry/publication and the new recovery action. Recovery stops an in-flight
old provider request, saves the preset through SQLite and finishes local OCR while
the synthetic description response remains held. The visible pipeline also
shows OCR completion and frame progress for the active descriptions. Releasing
that response lets
the visual replacement finish. The original color/test-pattern media contains no
readable text, so the summary journey consumes its baseline provider evidence
before local OCR replaces it; the separate throughput fixture proves real local
text recognition and subsequent search.

Browser layout cases cover light/dark at 1,440 and 390 pixels. The native recovery
screenshot was inspected at desktop size. These are partial native phases, not
the default capture/restart/keyboard suite; ambient capture devices, live models
and Vulkan were not exercised.

## Throughput measurement

The opt-in production-adapter benchmark passed its 15-minute processing budget:
45 minutes of generated 5120×1440 media produced 540 frame occurrences, 45 actual
Tesseract calls and 180 indexed evidence fragments in **518,614 ms (8 min 39 sec)**.
The fixture changes once per minute and includes English/German synthetic text,
ticket numbers, a meeting time and a monetary amount. Search found the amount
after publication. Source generation took about 52 seconds and is excluded from
processing time. This ran in the debug build with system `eng+deu` Tesseract and
the installed pinned E5 model on the real CPU worker, with two embedding threads.

Repeated screens exercise warm exact-image reuse within the run; this is one full
cold pipeline run, not a separate cold/warm workload comparison. OCR checkpoints
reached 37½ source minutes in 233,722 ms; the remaining time includes the final
OCR sections and CPU indexing. Two earlier diagnostic runs were deliberately
stopped to fix redundant full-resolution pixel comparison and decoding; they are
not successful throughput results. Real managed best-language assets took 2.4
seconds for a single synthetic bilingual screen in the separate microbenchmark.

The result proves the adapters and search publication on this fixture. It does
not predict the same time for continuously changing screens, smaller text,
different language assets or concurrent capture. CPU embedding remains a material
cost. Request batching submits multiple documents together; the shipped worker
still evaluates individual documents sequentially within a request.

## Ownership review

Chunk orchestration moved to `ocr_documents/chunk.rs` so extraction, recognition,
sampling and cache lifecycle have distinct owners. It is roughly 440 lines and
coordinates the established frame publication flow. Persistence and scheduling
remain below 500 lines. No broad lint or dead-code suppression was added.

Commands and benchmark prerequisites are maintained in
[the validation map](agent-validation.md). Reports under `/tmp/daydream-*` are
local diagnostics and are not shipped with the app.

Final commands (with `COREPACK_HOME=/tmp/corepack` and the authorized Cargo home):

- `corepack yarn check`: passed.
- `corepack yarn rust:fmt`: passed.
- `corepack yarn rust:clippy -- -D warnings`: passed.
- `corepack yarn rust:test -- --test-threads=2`: passed.
- The ignored `synthetic_45_minute_local_processing_benchmark`: passed explicitly.
- `DAYDREAM_E2E_PHASES='bootstrap inference' corepack yarn test:e2e:native`:
  fresh product build validated, then focused fixture corrections rerun with
  `--reuse-validated-build`; final bootstrap and inference phases passed.

Processing coverage is under `/tmp/daydream-processing-coverage`; settings
coverage is under `/tmp/daydream-settings-coverage`. Final command logs are
`/tmp/daydream-js-gate.log`, `/tmp/daydream-native-tests.log`,
`/tmp/daydream-native-clippy.log`, `/tmp/daydream-visual-benchmark.log` and
`/tmp/daydream-local-native-e2e.log`.
Final native screenshots and workflow reports are retained in
`/tmp/daydream-native-e2e.e51cD1`.

## Review follow-up

The four review findings are addressed at their owners:

- Explicit OCR retry timestamps override a newly computed adaptive omission.
  The decoded-screen regression carries a checkpoint from the preceding chunk,
  resets history, retries the failed checkpoint through native selection
  persistence, and verifies it still requires recognition.
- Legacy observation totals count all persisted checkpoints after queuing retry
  work, including retained successes. The activity regression observes 5/6 before
  the retried checkpoint completes and 6/6 afterward.
- Applying the local preset invalidates and reloads the cached settings, including
  error outcomes that may follow a persisted preset. Browser workflows reopen
  Settings and save an unrelated preference without undoing the preset.
- Recovery snapshots retain interrupted visual as well as nonvisual chunk IDs.
  Failed replacement admission restores both; successful admission restores only
  nonvisual work. A real SQLite scenario leaves an independent waveform job paused
  while checking restoration and pause intent for the interrupted owner.

The focused JavaScript run passes 145 tests with 100% measured branch coverage
(14/14) on the action hook. The full JavaScript gate passes 1,067 desktop tests,
nine shared tests and one FFmpeg test, plus lint and workspace typechecks.
The full locked native suite passes 717 library tests and three resource-staging
tests; five optional tests remain ignored. Native formatting also passes.
Strict all-target Clippy passes with `-D warnings`. Native checks use locked
Cargo with two build jobs; JavaScript checks use pinned Yarn through Corepack.
Rust branch instrumentation remains unavailable. The admission test uses a file
presence fixture because the paused job prevents media decoding; the real-media
recovery path remains covered separately by the Tauri journey.

The fresh frontend/custom-protocol build and actual Tauri bootstrap/inference
journey pass. The journey now loads Settings before recovery, reopens Settings
afterward and saves the debug preference, then checks through native IPC that
Tesseract, Fast resource use and adaptive sampling remain persisted. Its report
`/tmp/daydream-native-e2e.e4uvsB/local-visual-settings.json` records this outcome;
the same directory contains the recovery screenshot and frame progress report.
The description provider remains the controlled local HTTP fixture. No live
provider, private-recording or Vulkan checks were added in this follow-up.

Follow-up logs: `/tmp/daydream-review-js-gate.log`,
`/tmp/daydream-review-browser.log`, `/tmp/daydream-review-native-full.log`,
`/tmp/daydream-review-clippy.log`, `/tmp/daydream-review-build.log` and
`/tmp/daydream-review-native-e2e.log`. Source-mapped action-hook coverage is in
`/tmp/daydream-review-coverage`.

## Recovery and retry review regressions

Visual recovery now releases its action guard and pending state even when the
settings reload rejects. The settings store retains its read error, the card
preserves any preceding processing error, and a subsequent Pause action works.
The two Browser Mode regressions first failed with disabled controls and unhandled
settings-read rejections. They now pass through the real settings store with only
its IPC client substituted.

Observation retries retain the admitted job's sampling policy, recognition rate
and recording-sync choice while accepting current provider settings. The native
regression first loaded adaptive sampling instead of the saved full policy. It
now persists a failed 1,000 ms checkpoint, retries with current settings, claims
the work through the scheduler and verifies that frame preparation queues the
same checkpoint. It uses isolated SQLite and synthetic frame metadata; it does
not call a live model or decode a recording.

The focused processing run passes 134 tests. Source-mapped V8 coverage measures
all 16 action-hook branches (100%) in
`/tmp/daydream-review-recovery-coverage/coverage-summary.json`. Rust branch
instrumentation remains unavailable. Focused logs are
`/tmp/daydream-review-focused.log` and `/tmp/daydream-review-visual.log`;
pre-fix failures are recorded in `/tmp/daydream-review-browser-red.log` and
`/tmp/daydream-review-native-red.log`. No public contracts or stored formats
change in these fixes, and no dependencies are added. Separate manual visual,
live-provider and packaged-app checks were not repeated for these failure paths.

The final immutable Yarn install and `corepack yarn check` pass: lint, all
workspace typechecks, 1,145 desktop tests, nine shared tests and one FFmpeg test.
Lint retains the existing TanStack Virtual compatibility warning, and the broad
browser run emits existing ResizeObserver diagnostics without failing the gate.
Native formatting, strict all-target Clippy (`-D warnings`), 742 native library
tests and three resource-staging tests pass; seven optional native tests remain
ignored. Final logs are `/tmp/daydream-review-final-js.log` and
`/tmp/daydream-review-final-native.log`. Cargo checks use the lockfile and two
build jobs. Browser and loopback-dependent native checks ran outside the sandbox
after the initial browser run encountered a loopback bind denial.

No implementation owner changed for these fixes exceeds 400 physical lines.
Existing flat-folder density remains follow-up debt: frontend
`features/processing` has 17 direct TypeScript files; native
`ai/processing/visual` has 17 direct Rust files and `ai/processing/worker` has 19.
These fixes add no new source files or folder budgets.
