# Visual context and Studio read fixes

All four review findings have focused regression coverage:

- Visual context checks inherited dependency ranges and propagates the earliest
  dependency into new observations. Eligibility is checked again after image
  restoration and before publication. The SQLite tests cover an ancestor outside
  the three-image window, scoped redactions, safe restarts and gaps.
- Pruned context restoration decodes one source frame without the FPS filter or
  a 1ms extraction limit. Real pinned FFmpeg tests restore the selected stream of
  a synthetic 3fps video at 333, 667, 1000 and 1667ms, and cover cancellation.
- Waveform publications received during an unresolved read are retained. Pending
  or failed responses trigger one follow-up read; completed cache entries survive
  unrelated events. The original stuck-loading regression failed before the fix
  and passes afterward. Privacy, teardown, unavailable recordings, subscription
  failure and bounded viewport caching are also covered.
- Activity queries read the timeline and Review windows directly in SQLite.
  They return focus, window inventory, mouse and keyboard data from both windows,
  without fetching intervening events or duplicating overlaps. Existing full and
  single-window reads retain their behavior and privacy filtering.

Schema 39 conservatively expands historical visual dependency ranges to recording
start where the old rolling window could have lost lineage. Recordings, OCR and
observation content are retained. Earlier redactions can consequently withhold
more historical observations until explicit reprocessing rebuilds precise ranges.

## Verification

- `COREPACK_HOME=/tmp/corepack corepack yarn check`: passed lint, workspace
  typechecks, 1003 desktop tests, nine shared tests and one FFmpeg test.
- Native format and all-target Clippy: passed.
- Standard full native gate: 699 passed, four ignored, plus three resource-staging
  tests passed, using the default system FFmpeg.
- Focused native AI checks with pinned FFmpeg: 182 passed. Migration checks: 14
  passed, including historical preservation and schema version 39.
- Waveform browser suite: eight passed; source-mapped branch coverage 54/59
  (91.52%). Reports: `/tmp/review-waveform-coverage`.
- Native Tauri/WebKit bootstrap and inference phases passed against freshly built
  assets, including shared inference, partial retry and summary publication.
  The subsequent Studio workload phase stopped before Studio entry while waiting
  for the Library day list. Artifacts: `/tmp/daydream-native-e2e.MTLDLe`.

The full pinned-FFmpeg native run passed 698 tests and failed the separate
audio-cleanup playback test with an empty stream. That failure reproduced in
isolation with pinned FFmpeg and passed with the default system FFmpeg; playback
cleanup code was not changed. The standard full native gate passed with the
default system FFmpeg.

Browser IPC tests replace native transport and events. SQLite and FFmpeg
regressions use production implementations with synthetic data. Native inference
uses a local protocol fixture, not a live provider. Rust branch coverage and the
blocked native Studio workload were not established. Existing React Compiler and
Studio resize console diagnostics were not suppressed.
