# Capture production journey inventory

Scope: production `CaptureScreen` and `SourceCaptureWorkspace`; all fixtures synthetic. Browser coverage replaces only recorder IPC boundaries, preserving real UI/store transitions. Canvas scene presets/grid are legacy components outside the production screen; current output is one video per source, with crop and scale.

| Story/control | States and outcomes | Exact proof |
| --- | --- | --- |
| CAP-SOURCE: Add monitor sources | initial sources, duplicate portal identity confirmation/refresh, delete source, delete last source and save | `CaptureScreen.browser.test.tsx`: reviews selected sources; asks before refreshing a duplicate-looking monitor; deletes a draft source; deleting the last monitor |
| CAP-AUDIO: Add audio/source picker/Delete audio | optional desktop and microphone, available device choice, exhausted devices, persisted reload | same file: adds and deletes optional audio; saves and reloads selected audio; audio picker shows every unadded device; disables audio add |
| CAP-SAVE: Save | initial legacy review, dirty defaults, dirty geometry, saved reload, pending save, failure retaining edits, retry | existing reviews selected sources; saves shared frame interval; new `CaptureScreen.setup-journeys.browser.test.tsx` CAP-SETUP-01/02/03 |
| CAP-CROP: Capture area/Region/Crop X,Y,width,height/Full source/Reset | physical pixels, invalid crop remains editable, valid recovery, custom combined crop/output persists, full-source hides region controls | existing crops right half; invalid crop/reset; CAP-SETUP-01 |
| CAP-SCALE: Output scale 100%/50%/Custom size/Output width/Reset | aspect-ratio output, preview dimensions, persisted fresh load, reset | existing ultrawide half-scale; CAP-SETUP-01/04 |
| CAP-DEFAULTS: Frame interval/Video bitrate per source | real typing, dirty save required before start, locked while request/session active, saved defaults fresh reload, loading failure retry | existing shared settings; CAP-SETUP-01/02/03; CAP-LIFECYCLE-01 |
| CAP-PREVIEW: source selection and preview | live frame, startup saved scene, empty scene, delete last, re-entry/protection status, single backend request after stop | existing saved scene startup; live preview; empty saved scene; delete last; auto-preview after stop; protection reconciliation |
| CAP-START: Start | unavailable writer, stale portal permissions, dirty settings, pending/double-click lock, failed start recovery without picker | existing media writer blocker; stale screen permission; new `CaptureScreen.lifecycle-journeys.browser.test.tsx` CAP-LIFECYCLE-01/02 |
| CAP-SESSION: Pause/Resume/Stop | start→pause→failed resume→retry→stop, preserved recording ownership, pending double-click locks, active controls locked, protected-window reconciliation | CAP-LIFECYCLE-01; existing read-only while recording; protected-window pause |
| CAP-FINALIZATION | saving recording status, Start/config locked until event completion, preview resumes | existing saving banner while stop finalization finishing; auto-preview after stop |
| CAP-COMPACT: Source output sheet | narrow visible editing, keyboard open/selection/Escape, trigger focus restore, save after dismissal | existing theme/width matrix; CAP-SETUP-04 |
| CAP-FOCUS: Focus monitor/Delete selected | each selected monitor retains independent output, save both, delete selected and fall back visibly | CAP-SETUP-05 |
| CAP-FAILURE: Pause/Stop errors | recording stays controlled, settings stay locked, stop retry succeeds | CAP-LIFECYCLE-03 |
| CAP-PAUSE-SAVING: Resume/Stop | native paused response still has finalization; controls stay disabled until its completion event, then Resume sends one command | CAP-LIFECYCLE-04 |
| CAP-GLOBAL | global pause/resume/stop/duration/size | `recorder-session-controls.browser.test.tsx`, `recorder-status-bar.browser.test.tsx` |

Added stories use `userEvent` real browser typing/clicking/keyboard, visible outcomes, explicit deferred IPC settlement and actual store behavior. No private database/device access, ambient capture or fabricated native evidence.

Validation: targeted eslint --fix passed. Initial typecheck found readonly array fixture issue, corrected using explicit `RecorderPreviewStatus`; unrelated library peer errors reported to coordinator. First browser run: all five setup journeys passed. Typed native-error regression run: permission-start passed; dependency-resume and lifecycle-pause failed because CaptureScreen routed expected command failures to fatal error boundary and unmounted the controls. Evidence `/tmp/daydream-qa-capture-red.log` (2failed/1passed). CaptureScreen now keeps typed RecorderError failures inline (store already owns error message), while unexpected exceptions retain the existing boundary path. Store rejection contract/native pipeline unchanged. Final scoped browser gate PASS: existing CaptureScreen + setup journeys + lifecycle journeys, 39/39 tests across three files. Command: `COREPACK_HOME=/tmp/corepack corepack yarn workspace @daydream/desktop test:browser src/screens/CaptureScreen.browser.test.tsx src/screens/CaptureScreen.setup-journeys.browser.test.tsx src/screens/CaptureScreen.lifecycle-journeys.browser.test.tsx`. Evidence `/tmp/daydream-qa-capture-green.log`.

The running native app exposed an enabled Resume no-op: its pause response still
contained finalization, so the store refused an action the toolbar appeared to
allow. CAP-LIFECYCLE-04 reproduced that response and completion event through the
production store, Capture screen and global controls. The toolbar now uses the
shared lifecycle lock. All 40 Capture browser tests pass; native/store guards are
unchanged. Evidence: `/tmp/daydream-qa-capture-finalization-{red,green}.log`.

Native public-command scenarios cover composed, independent and sparse-cadence
synthetic capture through the real writer. The running-app harness separately
checks recording, media playback and persistence. Real portal/device capture
remains an operator boundary; no physical desktop capture was performed.
