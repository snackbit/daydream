# Processing overview verification

The Processing page and recording/day Studio share a recording pipeline with
separate OCR and observation stages. Timeline screen work uses one eligible,
continuous indicator across adjacent scheduling chunks. Processing remains native
owned; the interface reads bounded activity, source provenance, waiting reasons
and measured timing through the existing typed commands.

The group input adds optional `recordingIds`; group counts add running/queued
section totals over the complete scope. Lane activity is additive and contains at
most three running and three queued sections. Older responses without activity or
the new counts still decode. Unverified/redacted source labels and positions are
withheld. Schema v38 adds nullable phase/progress timestamps; old measurements stay
unknown, and unchanged progress does not update the measured-progress timestamp.

Recovery now supervises worker exits and keeps slots owned through source worker
completion. Completed parents with unfinished admitted sections resume at startup;
failed/cancelled parents remain stopped. Successful sibling stages retain their
outcome. Events and periodic/focus reconciliation cannot replace newer recording
state, clear Studio selection, or collapse expanded section pages.

Verification evidence (synthetic data only):

- Native full suite: 694 tests and three resource-staging tests passed; four
  existing explicit benchmarks remain ignored. Evidence:
  `/tmp/daydream-processing-native-final.log`.
- Native formatting and all-target Clippy passed. Evidence:
  `/tmp/daydream-processing-{fmt,clippy}.log`.
- Immutable Yarn installation passed. Evidence:
  `/tmp/daydream-processing-install.log`.
- The isolated Tauri/WebKit bootstrap and inference journey passed. It holds a
  local fixture-provider response while checking native OCR activity, observation
  dependencies, continuous source indicators and the Studio sheet, then exercises
  validation, retry and publication. The settled-sheet screenshot was inspected.
  Evidence: `/tmp/daydream-processing-e2e-final.log` and
  `/tmp/daydream-native-e2e.olMpF0/`.
- Focused browser workflows exercise both themes at 1440 and 390 pixels, scoped
  navigation, selection/focus preservation, transient errors, overlapping actions,
  lost subscriptions/events, stale reads and pagination recovery. Media/model
  boundaries in these tests are mocked; native provider integration is proved
  separately by the fixture journey.

The final `COREPACK_HOME=/tmp/corepack corepack yarn check` passed lint,
all-consumer typechecks, 937 desktop tests, nine shared tests and one FFmpeg test.
The last test-only extension passed targeted lint/typecheck and the focused suite
of 113 tests. Evidence: `/tmp/daydream-processing-js-final.log`,
`/tmp/daydream-processing-final-{test-lint,types}.log` and
`/tmp/daydream-processing-coverage.log`.

V8 instrumentation covered 426/435 frontend branches (97.93%) across sixteen
processing owners; each measured file exceeds 90% (minimum 91.3%). Reports:
`/tmp/daydream-processing-coverage/coverage-{summary,final}.json`. This owner set
does not include the broader Studio orchestration or imply repository-wide
coverage. Native source and direct Tauri producer/consumer paths were verified
separately; Rust branch instrumentation is unavailable on this host. Existing
React Compiler/ResizeObserver notices remain in the broader suite logs, which
completed successfully. No live model-quality or physical capture-device
verification is claimed.
