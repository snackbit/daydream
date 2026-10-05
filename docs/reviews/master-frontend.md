# Frontend master review: 2026-10-05

Read-only baseline: clean main `6527f5fb14435f52f2bd880dde0e9635eaa50506`.
This records the audit before fixes; findings are intentionally retained.
Sources: AGENTS.md, README, implemented M1/M2/M2.1 milestone requirements.
Planned M3 functionality was excluded.

| Priority | Category | Finding | Evidence and proposed correction |
| --- | --- | --- | --- |
| High | Security | Redaction leaves cached transcript/OCR/VLM text visible in Inspector. | `use-recording-studio-controller.ts:146` replaces detail only; `use-recording-visual-observations.ts:51` reloads on chunk changes only. Backend redaction commands emit no processing event. Invalidate in-flight reads, hide affected cached data immediately, and reload all derived state with failure coverage. |
| Medium | Logical Flaws & Bugs | Capture controls stay stale after protected-window automation changes recording state on another route. | `CaptureScreen.tsx:81` loads only when persistent store status is idle; subscription is removed on unmount. Reconcile status on return without overwriting dirty scenes, protecting snapshot/event ordering. |
| Medium | Logical Flaws & Bugs | Saving while editing overwrites newer scene/settings drafts and permits overlapping settings saves. | `recorder-workspace-persistence-actions.ts:63,102`, `settings-store.ts:143,165`. Preserve draft revisions and independent request ownership. |
| Medium | Code Quality & Folder Structure | Visual-frame construction repeatedly sorts all frames for every frame during playback rendering. | `studio-visual-observations.ts:45,114`, `studio-sidebar.tsx:73`. Production helper measurement: 1,000 frames 33 ms; 5,000 654 ms; 10,000 2,366 ms. Sort once and share a stable view model. |

## Route and coverage inventory

| Route | Screen and ownership | Browser coverage inspected |
| --- | --- | --- |
| `/` | HomeScreen, readiness overview | No dedicated Home workflow test. |
| `/capture` | CaptureScreen, recorder workspace, canvas/geometry, source/audio selection, toolbar/preview | Saving, layout/editing, duplicate monitors, permissions, preview, lifecycle/finalization, recording locks, audio. No remount missed-event or save/edit race coverage. |
| `/library` | LibraryScreen, overview controller/items/dialogs | Selection/views, unavailable/locked media, delete/rename/open. |
| `/library/$recordingId` | RecordingStudioScreen, route controller, player/media/redaction, timeline store/viewport/selection, Inspector | Playback/seek/pause gaps/rejections, stale sessions/routes, AI events, waveforms, editing, scissors/unredaction, activity/visual inspection. Redaction reload tests cover activity only. |
| `/settings` | SettingsScreen, controller/store, AI secrets/models, four tabs and protected-window picker | Save/reset, cleanup confirmation, secret references/rollback, sliders, permission repair, dirty navigation. No save/edit ordering coverage. |
| `/notifications` | NotificationsScreen, store/lifecycle/progress | Grouping, dismissal, retry acknowledgment, active downloads, empty state. |
| `/diagnostics` | DiagnosticsScreen, controller/store/sections | Writer/GPU capabilities, clipboard diagnostics/debug bundle. |
| Global | AppShell, startup gate/warnings, NotificationCenter, RecorderStatusBar, PageLayout | Startup/loading/error, popover, performance indicator/layout. No full-shell narrow navigation workflow identified. |

Unit coverage inspected: recorder lifecycle/preview/store/scene math/performance;
startup; settings; diagnostics; notifications; timeline store; IPC command shapes;
shared contracts/defaults; FFmpeg resolution. IPC mocks prove request shapes, not
native behavior or runtime decoding.

## Baseline validation

- `corepack yarn lint`: passed.
- `corepack yarn typecheck`: passed.
- Desktop Vitest unit project, two workers, external configuration/cache and
  disabled results cache: 18 files / 151 tests passed in 1.4 seconds.
- Unchanged production visual helper scaling measured using synthetic metadata.
- Browser execution, real-native interaction, packaged app and dependency
  advisory queries skipped during this audit. Browser tests were read.
- No repository edits, install, autofix, Git mutations or snapshot generation
  occurred during the read-only audit.

Limited-inspection concerns: rejected protected-window picker/status IPC and
secret deletion failure recovery. These were not promoted to confirmed findings.
Oversized owners identified for cleanup when touched: protected-window settings
card 614 lines, track-lane 495, library overview controller 415, scene layout
geometry 425.

## Implemented fixes and focused verification

The baseline findings above are preserved. Main now isolates result read ownership
in `features/ai/results/use-recording-result.ts` and source/timestamp privacy
filtering in `derived-result-visibility.ts`. Recording/redaction revisions hide old
results during render, invalidate pending reads and reload transcript, visual and
activity results. Failed reloads retain no data from the previous privacy revision.
The toolbar refresh also reloads derived results.

Capture subscribes before fetching current status on each mount; newer events and
actions take precedence over snapshots. This does not reload dirty scenes. Scene
and settings saves preserve newer drafts, serialize requests, and retain honest
request status. Settings controls remain disabled through secret preparation and
cleanup. Visual frames are sorted once and shared by timeline and Inspector.

| Protected scenario | Proof |
| --- | --- |
| Redact populated transcript/OCR with completed processing and failed reloads | Extended Studio Browser Mode workflow; includes opening the OCR dialog before cutting. |
| An old private result resolves after a redaction; current read fails | Result-hook Browser Mode regression. |
| Native transcript UUID differs from Library lane ID; partial overlap and exact boundaries | Pure visibility tests for transcript/OCR/VLM and lane-scoped activity. |
| Protected-window pause occurs while Capture route is absent | Capture remount Browser Mode regression; store test also checks event/snapshot ordering and dirty-scene preservation. |
| Edit during an outstanding save; duplicate submit | Deferred-request recorder/settings store regressions. |
| Frame ordering/grouped source regions and pause/segment clipping | Deterministic visual-frame tests and unchanged-helper before/after measurement. |

Focused checks on the upgraded graph: 4 unit files / 50 tests, 4 browser files /
95 tests, all workspace typechecks and scoped ESLint autofix passed. Logs:
`/tmp/daydream-frontend-green-{unit,browser,typecheck}.log`.

The same synthetic performance measurement after the change: 1,000 frames 2 ms;
5,000 frames 4 ms; 10,000 frames 12 ms, versus 2,366 ms at 10,000 before. These
timings measure frame-model construction, not native decoding or total rendering.

All touched production owners remain below 400 physical lines. New results hook
65 lines, visibility mapping 175, studio route controller 322, recorder persistence
195, lifecycle 193, settings store 231, settings controller 277. Existing
untouched protected-window settings and track-lane owners remain follow-up debt.

Existing dense directories touched by this batch remain compatibility/deferred
cleanup boundaries: `screens` 14 direct TS/TSX files, `features/recorder` 19,
`features/settings` 12, and `features/library/components/studio` 42. The new
results owner has four files. For this bounded fix the studio directory has an
inclusive budget of 42 direct files: the new frame-model regression stays beside
its existing owner and imports remain stable. Reconsider that budget before adding
further implementation; the next useful split is visual markers/track-lane media
ownership. The other dense directories receive no new files in this fix and
remain follow-up debt.

Full JavaScript gates initially stopped at newly flagged formatter-only unions
after dependency refresh. A coherent root `corepack yarn lint:fix` batch passed;
the inspected additional shared changes only reformat unions, with no contract
or runtime change. The final `corepack yarn check` passed: lint, all workspace
typechecks, 31 desktop files / 280 tests, 9 shared tests and 1 FFmpeg test (290
tests total). Log: `/tmp/daydream-main-js-gate.log`.

`corepack yarn workspace @daydream/desktop build` also passed on the final
dependency graph. Vite reported the existing bundled-route chunk size warning
(859 kB / 256 kB gzip); route imports remain eager. This is a bundle cost, not a
new runtime failure. Log: `/tmp/daydream-main-build.log`.

One final gate launch hit automatic approval-review model capacity and was not
executed. An unchanged retry succeeded; no approval gap remains.

Browser checks use synthetic data and mocked
IPC; real Wayland portals, recording devices, WebKit and packaged native behavior
were not manually tested by this frontend fix batch.
