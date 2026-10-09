# Production user stories and validation

This record maps production features to user actions, state transitions and
automated proof. Test names describe outcomes; coverage is assessed against the
feature inventory, not a line-coverage percentage.

## Test boundaries

| Layer | What it proves | External boundary |
| --- | --- | --- |
| Browser journeys | Actual screens, routing, controls, stores, typed clients and visible recovery | Tauri transport, time and browser media APIs use synthetic fixtures. |
| Native journeys | Running Tauri UI, command handlers, migrations and durable settings in isolated app data | GTK Broadway uses a test-only fallback for unset display DPI. |
| Native scenarios | Recorder public commands, real media writer, storage, privacy, processing and Recall invariants | Synthetic capture and bounded local provider fixtures replace devices and remote services. |
| Model proofs | Actual managed CPU/Vulkan tokenizer, inference and retrieval behavior | Pinned model files and synthetic passages. |

Real desktop portal permission dialogs, microphone hardware, compositor behavior
and distribution installers require operator checks. Automated capture uses
synthetic media and must never start ambient screen or microphone recording.

## Scenario selection

For each reachable production action, cover its ordinary outcome and relevant
failure/recovery behavior. Stateful journeys also cover saved versus dirty data,
pending duplicate actions, navigation and restart, unavailable dependencies,
privacy invalidation and late responses. Layout and keyboard scenarios cover
both themes and narrow viewports. Impossible combinations are excluded by the
production state machine; independent visual permutations use representative
combinations rather than repeating every workflow at every viewport.

## Feature inventory

The detailed inventories map reachable production controls to exact test titles,
including reused tests and new multi-step journeys:

- [Capture](testing/capture-journeys.md)
- [Settings, startup and utilities](testing/settings-journeys.md)
- [Studio](testing/studio-journeys.md)
- [Library, Recall and navigation](testing/recall-journeys.md)
- [Running native app](testing/native-journeys.md) and [runner prerequisites](../apps/desktop/scripts/native-e2e/README.md)

| Story family | Production feature |
| --- | --- |
| NAV | Sidebar, mobile navigation, appearance, startup warnings and route transitions. |
| CAP | Source selection, crop/scale, audio, sampling/quality, preview and recording lifecycle. |
| STU | Playback, source/grid views, frame navigation, timeline selection, redactions and inspectors. |
| SET | General, Record, Storage/Library, AI profiles, model cache, secrets and settings persistence. |
| PROC | Task filters, pagination, details and pause/resume/retry/cancel actions. |
| NOT | Notification actions, dismissal, history and recovery. |
| DIAG | Health checks, raw details, clipboard/export and recovery. |
| LIB | Days, Today, Recordings, thumbnails and recording management. |
| SRCH | Text/hybrid search, scope, pagination, cancellation and source navigation. |
| REC | Day generation, citations, review, corrections, feedback, suggestions and local tasks. |

## User stories

| Stories | User outcome and required recovery |
| --- | --- |
| CAP-SETUP-01…05 | I can select independent monitors, crop/resize each one, save and reopen my setup. Failed saves retain edits; narrow editing remains keyboard reachable. |
| CAP-LIFECYCLE-01…04 | I can start, pause, resume and stop. Pending commands and segment finalization disable actions; expected failures preserve controls and permit retry. |
| SET-01…02 | I can download, cancel and delete a model. Failed deletion leaves the model available until a successful retry. |
| SET-03…05 | I can save preferences and confirm or cancel resets/retention cleanup. Cancel never persists or deletes my data. |
| SET-06…07 | I can create, validate, edit and remove protection rules, or recover an unavailable window picker. |
| SET-08…11 | I can choose timing, resource presets, models and available backends. Remote embedding consent is independent of generation consent. |
| SET-12…14 | I can recover startup or Settings loading failures through visible retry controls. |
| PROC-01…06 | I can filter/paginate work, inspect details, pause/resume/cancel, and retry a selected failed unit. Failed reads/actions preserve available work. |
| NOT-01…03 | I can retry actions or dismiss completed notifications. Failures remain visible; ongoing work cannot be cleared accidentally. |
| DIAG-01…03 | I can inspect or copy diagnostics. Failed reads/copies can recover and never copy partial output. |
| STU-01…02 | I can rename or delete a recording, cancel safely, and retry an error inside its dialog. Pending deletion cannot issue twice. |
| STU-03…06 | I can control independent audio, inspect debug data, skip within bounds and reorder lanes with the keyboard. |
| STU-07…11 | I can create/unhide cuts and retry failures. Older completions cannot clear a newer range or mask selection. |
| STU-12…14 | I can skip to eligible media and recover playback or preparation failures without unintended autoplay. |
| LIB / NAV | I can move among Days, Today, Recordings, Search and Settings; URL/history transitions preserve the intended screen and query. |
| SRCH | I can search literal text or meaning, scope results, paginate in both directions and open an eligible source. Privacy changes invalidate stale results immediately. |
| REC | I can correct sessions, rate interpretations, accept/dismiss suggestions and edit/done/reopen/dismiss tasks. Edits survive navigation; hidden source-derived content stays withheld. |
| NATIVE | I can save/discard/reset settings across a real process restart, record synthetic sources through the real writer, then open finalized media in Studio. |

## Verification status

The combined gate's JavaScript phase passes 522 tests across 65 files: 512 desktop,
9 shared and 1 FFmpeg. Native formatting, warning-free Clippy and 492 tests pass;
three explicit performance benchmarks remain excluded from normal test runs.
The complete `yarn test:e2e` command exits 0, including a fresh production build
and all 11 unique native stories (13 phase entries): actual decoded playback,
restart persistence and keyboard Search. There are 57 added browser journeys.
Evidence: `/tmp/daydream-production-e2e-verified.log` and
`/tmp/daydream-native-e2e.3eXHeR/`.

The full browser collection still logs one ResizeObserver teardown error/console
pair without failing Vitest; it was not suppressed. Private headless D-Bus also
logs unavailable systemd services. These runs do not replace real hardware,
permission, native-zoom or installer checks.

The initial native screenshot failed because Broadway reported an unset display
resolution: DPR was negative and computed sizes were invalid. A temporary GDK
getter shim forwards valid resolution and maps only unset/nonpositive resolution
to 96 DPI. With that test display correction, the unchanged Tauri app reported
1200 × 760, DPR 1, 16px root/14px body/36px heading, a 216px sidebar and a 64px
header. Its fresh-data smoke check remained idle with zero recordings, Recall
jobs or queued downloads. This verifies the headless native shell, not hardware
capture or distribution packaging.

## Running the suite

The feedback upgrade adds these scenarios to the existing production journeys:

| Story | User outcome and owning proof |
| --- | --- |
| MEDIA-STOP | A wide, low-FPS recording with separate audio tracks finishes. A blocked EOS/Null operation has a bounded failure, preserves files, and cannot publish late success. Real child-process, FD, startup-thread and parent-death cases are documented in [writer validation](recorder-media-worker-validation.md). |
| MODEL-TRANSFER | Embedding admission survives migration and rolls back failed notification persistence. Pause flushes partial data, releases the slot, and permits FIFO resume; HTTP ranges and checksums are checked. Settings and shared rows keep failures beside their actions and offer retry/refresh. |
| PICKER-OVERLAY | Users search and select a left-aligned window row. Pointer/keyboard selections dismiss dropdowns and return focus. Light/dark layouts are inspected at desktop and narrow widths; local icon resolution has a separate GTK smoke check. |
| MAINTENANCE | Each cleanup scope requires review and confirmation. Interrupted cleanup blocks new work and can resume after restart. Preferences preserve the library across restart, and explicit settings save authorizes retention again. SQLite compaction, source privacy and stale client responses have synthetic failure coverage. |
| RECORDING-GROUPS | Users see complete current lanes per recording, inspect paged sections, and retry one lane without changing siblings. Filtering/counts precede pagination; a 200-recording fixture verifies that only requested cards probe media. |
| PARTIAL-RECOVERY | Recovery creates a separate copy, preserves originals and privacy ranges, validates stream/PTS provenance, and requires manual processing. Ambiguous older stream metadata is rejected rather than guessed. |

Factory-reset filesystem/database/key operations use synthetic fixtures or mocked
external boundaries. The real UI reset journey exercises preference reset and
preserved synthetic recordings; it never deletes the user's Secret Service key.
These checks do not claim every hardware/OS permutation or 100% coverage.

The feedback verification completed with 542 workspace JavaScript tests, the full
548-test native run and three staging tests, then the narrow nullable-metadata
regressions and actual GTK icon smoke. Fresh-build native UI journeys pass all
11 unique stories / 13 phase entries, including preference reset with retained
playable recordings. Evidence and deliberate omissions are recorded in
[the refactoring ledger](refactoring-tasks.md#completed-feedback-upgrade).

From the repository root:

```sh
COREPACK_HOME=/tmp/corepack CARGO_HOME=/tmp/daydream-cargo-home \
  corepack yarn test:e2e
```

This runs the JavaScript lint/typecheck/unit/browser gates, native formatting,
Clippy and scenarios, then the real Tauri journeys. Native UI tests build fresh
frontend assets and the desktop binary by default. The native runner documents
its Linux prerequisites and synthetic
capture boundary in [its README](../apps/desktop/scripts/native-e2e/README.md).
Install a matching WebKitWebDriver on `PATH`, or select its verified binary with
`DAYDREAM_WEBKIT_WEBDRIVER`; native keyboard uses the standard automation protocol.
An explicit reuse flag is available only for iteration on an already validated
build. Missing prerequisites fail the run rather than silently skipping proof.

Use `yarn test:browser` or `yarn test:e2e:native` for a focused journey layer. See
[validation commands](agent-validation.md) for resource limits and scoped commands.
