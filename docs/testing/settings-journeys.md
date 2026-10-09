# Settings, Processing, Notifications and Diagnostics QA

All fixture content is synthetic. New tests use real components/controllers/stores; IPC clients, notification loading, router leave boundary and clipboard are external boundaries. Native window enumeration, filesystem model actions, permissions and real remote services remain native/manual boundaries.

## New user journeys

| Story | Test file and exact test prefix | Actions and observable result |
| --- | --- | --- |
| SET-01 | `features/settings/testing/ai-model-workflows.browser.test.tsx` — SET-01 | Download → Cancel → Download → Delete; native model states refresh visible actions and notification history. |
| SET-02 | Same — SET-02 | Failed Delete keeps downloaded model, retry removes it, clears error. |
| SET-03 | `features/settings/testing/settings-workflows.browser.test.tsx` — SET-03 | Expert/debug checkbox → Save → unmount → fresh load; settings remain checked. |
| SET-04 | Same — SET-04 | Dirty debug checkbox → Reset → Cancel; no preview/reset IPC, draft remains. |
| SET-05 | Same — SET-05 | Retention input → Save → cleanup Cancel → Save → Delete and save; only confirmed request persists. |
| SET-06 | Same — SET-06 | Protection enable/Add rule/label → matcher-required disabled Save → app matcher/Redact → Save/reopen → Delete rule/Save. |
| SET-07 | Same — SET-07 | Pick window unavailable → Refresh → choose synthetic app → Save; app identity/process rule persists. |
| SET-08 | Same — SET-08 | Processing timing Manual/After stop/During recording and resource Fast/Background → Save; presets persist individually. |
| SET-09 | Same — SET-09 | Remote embedding provider → test disabled until consent → endpoint/model/dimensions/convention/anonymous auth → synthetic test → Save; generation stays disabled. |
| SET-10 | Same — SET-10 | Local transcription model/language/Vulkan backend selection → Save; unavailable CUDA option is disabled. |
| SET-11 | Same — SET-11 | Local Recall BGE model/Auto backend → Save; readiness rejection → Check readiness → saved configuration Ready. |
| SET-12 | `features/startup/components/app-startup-gate.workflows.browser.test.tsx` — SET-12 | Native startup error → Retry rejection → Retry success → workspace opens using real startup store. |
| SET-13 | Same — SET-13 | Ready with optional-model warning → workspace opens and warning stays visible. |
| SET-14 | `features/settings/testing/settings-workflows.browser.test.tsx` — SET-14 | Initial settings load failure → retained shell/error → Try again → workspace preferences recover. Red observed for missing recovery control plus unhandled load rejection; minimal fix verified green. |
| PROC-01 | `features/processing/testing/processing-workflows.browser.test.tsx` — PROC-01 | All/Active/Needs attention/History tabs → Load earlier work → Refresh; loaded history survives current-page updates. |
| PROC-02 | Same — PROC-02 | Pause → Resume → Cancel; visible native lifecycle changes, cancelled task remains retryable. |
| PROC-03 | Same — PROC-03 | Rejected Pause → visible error → retry → Resume appears. |
| PROC-04 | Same — PROC-04 | Rejected pagination → loaded tasks remain → same-cursor retry → earlier work appears. |
| PROC-05 | Same — PROC-05 | Details rejection → Close/reopen → selected unit Use current settings rejection/retry → Close. |
| PROC-06 | Same — PROC-06 | Model task Cancel → Retry failed routes only through model client, never recording-job actions. |
| NOT-01 | `screens/NotificationsScreen.browser.test.tsx` — NOT-01 | Permanent dismiss rejection → visible history/error → retry → empty state. |
| NOT-02 | Same — NOT-02 | Model Retry rejection → action/history remains → retry succeeds without deletion. |
| NOT-03 | Same — NOT-03 | Clear rejection → retry clears completed history only; active download remains undeletable. |
| DIAG-01 | `screens/DiagnosticsScreen.browser.test.tsx` — DIAG-01 | Copy JSON clipboard rejection → visible status → retry copies loaded facts. |
| DIAG-02 | Same — DIAG-02 | Copy bundle IPC rejection → no partial clipboard write → retry copies bundle. |
| DIAG-03 | Same — DIAG-03 | Raw diagnostics disclosure opens/closes; retained facts remain without new device probes. |

## Existing control proof reused

| Production control group | Exact existing browser proof |
| --- | --- |
| Record cadence mode/frequency/bitrate, storage retention/size, Save, Reset, Reset settings | SettingsScreen — `SettingsScreen edits record and storage settings then resets them` |
| Cleanup Delete and save | SettingsScreen — `SettingsScreen confirms cleanup before saving deletion-causing storage settings` |
| Transcription provider/URL/key/model Save and rejected Save | SettingsScreen — `SettingsScreen saves remote transcription settings and the new key in one request`; `SettingsScreen retains the key draft when the atomic save fails` |
| Vision first profile/new key, observations toggle, OCR engine switch | SettingsScreen — `SettingsScreen saves a first Vision profile and its new key together`; `turning off screen observations keeps the Vision OCR endpoint and saved key`; `switching OCR back to Tesseract leaves the configured Vision endpoint intact` |
| Expert controls, language/word timestamp checkbox, processing/activity sliders, permission Repair | SettingsScreen — `SettingsScreen reveals expert transcription controls from General settings`; `SettingsScreen toggles checkbox fields from their description text`; `SettingsScreen saves AI slider values`; `SettingsScreen repairs Linux input activity permissions from AI settings` |
| Dirty leave Stay/Discard/Save and cleanup before leaving | SettingsScreen — `SettingsScreen blocks dirty leave and lets the user stay`; `SettingsScreen discards dirty settings before leaving`; `SettingsScreen saves dirty settings before leaving`; `SettingsScreen waits for cleanup confirmation before leaving after save` |
| Generation enable/endpoint/model/auth/Test, failure/retry | SettingsScreen — `Recall generation requires its own consent and tests anonymous profiles with synthetic text`; `Recall provider test failure preserves draft edits and remains retryable` |
| Vision Test and anonymous/stale draft behavior | vision-provider-test — `tests an unsaved Vision profile and displays its capability result`; `anonymous provider tests omit a previously entered key draft`; `changing the profile during a test hides its stale response` |
| Processing Details/Retry section/Retry with current settings/Reprocess audio/settings links | ProcessingScreen — `task details retry only the selected failed frame while completed sections stay available`; `failed work links to its configuration and retries with current settings`; `older recordings use the new speech detector only through explicit audio reprocessing` |
| Notification bell/acknowledge/action/Dismiss/load Retry and download Cancel | notification-center — `NotificationCenter opens, runs an action, and dismisses notification`; `NotificationCenter exposes a visible retry after local history load fails`; `NotificationCenter keeps queued downloads cancellable before models load` |
| Notification history permanent dismiss/Clear/load Retry | NotificationsScreen — `NotificationsScreen permanently dismisses one or all notifications`; `NotificationsScreen retries a failed history load and shows recovered updates` |
| Diagnostics Refresh/Retry/Copy JSON/Copy bundle | DiagnosticsScreen — `DiagnosticsScreen copies raw diagnostics JSON to the clipboard`; `DiagnosticsScreen copies debug log bundle to the clipboard`; `DiagnosticsScreen keeps its shell and retries after the initial device probe fails`; `DiagnosticsScreen preserves loaded facts and exposes retry after refresh fails` |

SettingsScreen.browser.test.tsx already covers Record cadence radio modes/input, bitrate, retention/size, Save/Reset/Reset settings, destructive cleanup confirmation, AI provider/model credentials, atomic save failures, description checkbox clicks, expert controls, activity permission Repair, AI sliders, dirty leave Stay/Discard/Save, and generation consent/auth/test retry. Vision OCR selection and observations toggle preserve endpoint/key. Settings store tests prove single-request saves preserve newer edits and atomic Recall credentials.

Vision provider dedicated browser tests cover Test unsaved draft, anonymous auth omitting entered key, and profile-change stale response suppression. Existing SettingsScreen light/dark 1440/1024/768/390/320 tests check Vision/Recall/settings section controls remain reachable.

ProcessingScreen.browser.test.tsx covers job Details, selected failed-unit Retry section (saved settings), Retry with current settings, failed OCR settings link, Open recording link construction, Recall-scoped Cancel, Refresh recovery, explicit Reprocess audio tracks, and light/dark recovery controls at five widths.

NotificationsScreen and notification-center.browser.test.tsx cover empty/populated history, permanent dismiss, Clear, notification bell open/acknowledge, notification action, Dismiss, View all navigation boundary, load Retry, ongoing downloads unread/undeletable, queued downloads cancellable and five-width light/dark controls.

DiagnosticsScreen.browser.test.tsx covers initial and subsequent Refresh failure/retry, Copy JSON, Copy bundle, loaded capability facts, and five-width light/dark actions. New tests use the real debug preference store/hook.

Startup app-startup-gate.browser.test.tsx currently mocks the startup store and covers loading, ready rendering and Retry invocation after a hard failure. SET-12/13 now prove real-store failure→ready recovery and non-blocking warning rendering. The warning banner has no action controls.

## Coverage limits and remaining control permutations

The grouped matrix covers selected control/risk combinations, not every Cartesian
combination. Disabled-debug Diagnostics navigation and appearance are covered by
the actual router and AppShell tests. Real portal/window metadata, input group
repair, model files and packaged WebKit require native or operator proof.

## Regression found

Notification history Dismiss/Retry/Clear handlers previously ignored promises that reject after the store exposes an error. New failure workflows observed unhandled promise rejections. Handlers now consume those already-reported failures at the UI event edge, preserving the store's rejecting API and visible recovery state.

Protected-window rule label/app/process text inputs, action/title selects, and dwell/merge number inputs previously had visible labels without associated IDs. SET-06/07 found unnamed controls through real browser accessible queries. Each owning field now uses useId and htmlFor/control ID association.

The affected Number/Text/Select field components were extracted into `features/settings/components/protected-windows/` because the parent had exceeded 600 physical lines before this fix. Parent ownership remains protection rules/picker/lifecycle at 525 lines; broader orchestration/editor/picker splitting is existing source debt. New workflow tests and shared fixture live under feature `testing/` folders rather than adding more direct files to dense screen/component folders.

Initial settings IPC load failure previously produced an unhandled promise rejection and a permanent unavailable view with no recovery action. SET-14 reproduced both failures. The controller now consumes its already-reported load failure and exposes an explicit retry; Settings renders Try again only for the unavailable initial state. No store/native contract changes.

## Verification

Targeted eslint autofix and desktop typecheck pass. Root coordinates the final global gate. First sandbox browser attempt could not bind loopback (EPERM); elevated batches reached tests. Browser regressions observed then corrected: uncaught notification failures, unnamed protected-window fields, and missing settings-load retry. Third expanded batch passed 71/72 with only a Close test-locator ambiguity; focused processing/settings follow-up passed 15/15 after field extraction and locator correction (`/tmp/daydream-qa-settings-final.log`). SET-14 red reproduced the unavailable-state failure (`/tmp/daydream-qa-settings-initial-red.log`); the final Settings workflow batch passed 10/10 with no unhandled errors (`/tmp/daydream-qa-settings-initial-green.log`). All 26 new stories pass across these settled focused runs.
