# Native master review baseline

Reviewed immutable commit: `6527f5fb14435f52f2bd880dde0e9635eaa50506`.
Scope: entire current repository, native recorder ownership. Main working tree
was clean during the read-only audit. These are pre-existing findings.

| Priority | Category | Baseline finding and evidence | Required fix |
| --- | --- | --- | --- |
| High | Security | Protection writes fail open: `recorder/window_protection.rs:266-281,309-319`; stop ignores closure/materialization errors before processing in `recorder/commands/lifecycle_commands.rs:334-339,363-364`. AI enqueue loads redactions only (`ai/processing/worker/enqueue.rs:29-35`). | Stop unsafe capture/resume and prevent processing until durable protection is materialized. |
| High | Security | Provider failure and no focused window both return `None` (`recorder/platform/linux/window_provider.rs:28-30`), which closes protection and resumes (`recorder/window_protection.rs:119-122,145-158`). | Distinguish unknown focus from reliable nonmatch and fail closed on metadata failure. |
| High | Logical Flaws & Bugs | Active worker errors are seen only in join/stop (`recorder/capture_runtime/worker.rs:22`, `recorder/media_writer/session.rs:114,269`). Recorder status never observes active failures. | Supervise workers and media bus; stop resources and emit/persist failure promptly. |
| Medium | Alignment with Authoritative Documentation & Scope | Changing protected rule IDs closes the old span and auto-resumes even while another protected window matches (`recorder/window_protection.rs:128-142`). Violates `docs/protected-window-recording-rules.md:143`. | Keep protection continuous across rule changes. |
| Medium | Alignment with Authoritative Documentation & Scope | Automatic pause ownership survives user Resume/Pause (`recorder/window_protection.rs:174-176,153-156`); an existing pause cannot be claimed manually (`recorder/manager/stop.rs:27-31`). Violates protected-window manual precedence at documentation line 144. | Manager owns pause cause; manual lifecycle actions revoke automatic resume authority. |

## Audit evidence and limitations

Inspected recorder lifecycle/preview, PipeWire intake/scheduling, CPU compositor,
DMA-BUF import/readback, GStreamer writing/probing, portal selection/tokens,
Linux audio/input/window providers, unsupported adapters, FFmpeg resolution,
capture-facing shared contracts, Tauri config/capabilities, and relevant tests.
Authoritative context: AGENTS.md, README.md, recorder milestone, protected-window
rules. Stale scaffold claims and deferred physical media cutting were accounted for.

Checks: static source/caller/test inspection, read-only Git status and HEAD.
No builds, runtime tests, capture sessions, installs, formatting, or writes ran
during the audit. Protected-window baseline tests cover one matcher scenario;
monitor/lifecycle timing scenarios are absent. Real devices, packaged artifacts,
DMA-BUF-only streams and audio/video synchronization were not validated.

## Fix evidence (separate from immutable baseline)

Implementation started after the read-only audit was complete and authorized.
Status: five baseline findings implemented and verified.

- Protection monitoring uses a typed status-bearing snapshot. Unknown metadata
  fails the recording; KWin compositor-owner loss invalidates its cached bridge.
- Protection spans remain continuous across rule changes. A pause action wins
  until every protected match has cleared for the merge gap.
- Span closure and all-track redaction materialization are atomic. Recording
  completion stops capture, joins monitoring, materializes spans, then
  publishes completed metadata. Any barrier failure marks the recording failed.
- The recorder manager owns manual/automatic pause authority. Manual pause can
  claim an existing or finalizing automatic pause and cancel an in-flight auto
  resume before segment activation. Canceled segment indexes are not reused.
- Active capture workers and the GStreamer bus/frame writer are supervised at
  250 ms intervals. Failure is published before resource cleanup; a failure
  during pause/stop/resume cannot be overwritten by a successful transition.
- Split matching, monitor state, persistence, lifecycle attachments/completion,
  runtime supervision, PipeWire intake and KWin script generation by ownership.

Regression coverage: rule changes/dwell/merge boundaries; unknown provider;
failed span creation and closure; failed redaction materialization rollback and
retry; stop barrier failure; manual pause claims and Resume/Pause cycle;
in-flight auto-resume cancellation; failure during finalization; live capture
shutdown and failed persisted metadata; asynchronous media bus/frame-writer
failures without private diagnostics.

Validation evidence:

- Initial red regression command could not compile because global Cargo cache
  writes were denied. Escalation review infrastructure also failed (capacity).
  No behavior-level red result is claimed. Writable `/tmp/daydream-cargo-home`
  subsequently supplied by the coordinator resolved the cache issue.
- First focused `cargo test ... recorder::`: crate compiled, 130 passed, five
  new cases failed in fixture setup because a recording ID was not a UUID.
  The fixture now obeys the actual SQLite constraint. This was not an intended
  red behavior result.
- Second focused `CARGO_HOME=/tmp/daydream-cargo-home CARGO_BUILD_JOBS=2 cargo
  test --manifest-path apps/desktop/src-tauri/Cargo.toml recorder:: -- --nocapture`:
  137 passed, no warnings; log `/tmp/daydream-native-tests.log`.

Cross-review integration refinements after that focused run:

- Removed a premature command-layer monitor join. The manager alone owns the
  capture-stop → monitor-join → protection-barrier → completed-metadata order.
  Added an observable resource-order regression with a real GStreamer writer.
- Resume preparation recovery now consumes any deferred privacy failure instead
  of hiding it behind Paused. Both user and automatic resume paths emit the
  resulting status. Added the provider-loss-during-Selecting regression.
- Segment metadata write failure now aborts remaining resources and persists
  failed recording/segment states for both Pause and Stop. Added a SQLite trigger
  failure regression. Start/resume metadata creation is transactional, and
  cancellation/error paths publish terminal state even if secondary writes fail.
- Deferred automatic resume carries the manager's pause generation. A newer
  protected/manual pause revokes old requests and cancels a preparing segment.
  Added deferred authority and manual/protected cancellation scenarios.
- Split pause lifecycle from stop/finalization and split lifecycle failure tests
  from protection authority tests.
- Authenticated malformed/oversized KWin metadata invalidates the cached
  snapshot instead of silently preserving stale Available metadata. Untrusted
  tokens cannot invalidate that cache; regression covers both boundaries.

Final coordinated native gates after dependency updates and processing fixes:

- `CARGO_HOME=/tmp/daydream-cargo-home CARGO_BUILD_JOBS=2 cargo test --locked
  --manifest-path apps/desktop/src-tauri/Cargo.toml -- --test-threads=2`: **349
  passed**, no warnings or failures. Authorized escalation allowed temporary
  loopback test servers. Log `/tmp/daydream-native-full.log`.
- `CARGO_HOME=/tmp/daydream-cargo-home cargo fmt --manifest-path
  apps/desktop/src-tauri/Cargo.toml -- --check`: **passed** after the processing
  owner formatted its last two eligibility guards. Log
  `/tmp/daydream-native-fmt.log`.
- `CARGO_HOME=/tmp/daydream-cargo-home CARGO_BUILD_JOBS=2 cargo clippy --locked
  --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets -- -D warnings`:
  first pass reported ten diagnostics. Native fixes cover two equivalent boolean
  simplifications, a borrowed `Path` argument, empty-string checking, explicit
  native-word widening on 32-bit X11 targets, and test-module placement. The
  processing owner is fixing three oversized argument lists and a borrowed-slice
  test fixture. **Final repeat passed with no warnings**. Log
  `/tmp/daydream-native-clippy.log`. The processing owner's later failed-parent
  cancellation refinement was verified by its focused 94-test ownership run;
  the full 349-test suite was not repeated for that bounded change.

Residual limits: no real screen/microphone/PipeWire failure injection, packaged
artifact smoke test, DMA-BUF-only portal session or audio/video alignment check.
The provider-failure behavior is deliberately terminal for the current recording;
the user can repair permissions/provider availability and start a new recording.
V1 redactions continue to preserve original media bytes as documented.

## Additional pre-existing issue: input toggle classification

This follow-up was found after the original fourteen-issue cleanup and is
separate from the five immutable native baseline findings above. It was present
in the committed cleanup baseline `b5ca37d` at
`recorder/activity_input.rs:265-268`.

Priority: Medium. Category: Security. With keyboard activity enabled and mouse
activity disabled, `BTN_*` presses fell through the disabled mouse branch and
became keyboard activity. Mouse-button timing was therefore recorded despite
the mouse capture toggle being off.

Fix: classify button events first and consult only the mouse toggle for them.
Nonbutton keys consult only the keyboard toggle. This is a small correction at
the existing classifier boundary; it adds no capture state or persistence flow.

Replaced the constant-only placeholder test with two pure regressions using real
evdev `KeyEvent`/`InputEvent` metadata. Coverage includes all keyboard/mouse toggle
combinations, button and keyboard presses, repeats, and excluded releases.

- Red: `CARGO_HOME=/tmp/daydream-cargo-home CARGO_BUILD_JOBS=2 cargo test --locked
  --manifest-path apps/desktop/src-tauri/Cargo.toml evdev_capture_toggles --
  --test-threads=2` failed for the intended disabled-mouse case:
  `Some("keyboard-activity")` instead of `None`; the keyboard case passed.
  Log `/tmp/daydream-native-input-red.log`.
- Green: the same focused command passed both tests without warnings.
  Log `/tmp/daydream-native-input-green.log`.
- Targeted `rustfmt --edition 2021 --config skip_children=true --check
  apps/desktop/src-tauri/src/recorder/activity_input.rs`: passed.
- `CARGO_HOME=/tmp/daydream-cargo-home CARGO_BUILD_JOBS=2 cargo clippy --locked
  --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets -- -D warnings`:
  passed without warnings; log `/tmp/daydream-native-input-clippy.log`.
  No new full-suite run was requested
  for this bounded classifier follow-up. No real input devices were opened.
