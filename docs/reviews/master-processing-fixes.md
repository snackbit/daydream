# Processing privacy fixes and verification

The original findings remain in `master-processing.md` without retrospective edits.

Implemented source-role/stream-index mapping for Library redactions, including legacy UUID targets; current dispatch/execution/persistence eligibility; fail-closed protected-span guards; visible-only VLM context; restart reconciliation across parent job states; shared recording/job artifact deletion; startup scratch cleanup; and recording mutation leases. Cuts cancel work and wait for old worker leases before succeeding. Completed derived data remains stored and returns after explicit unredaction. Native lifecycle and frontend cached-result fixes have separate owners.

Storage recovery and process-local leases require a single owner of the app data directory. `recording-data.lock` uses the OS advisory exclusive lock from Rust's [File::try_lock](https://doc.rust-lang.org/std/fs/struct.File.html#method.try_lock), stabilized in Rust 1.89. Resolved dependencies already require Rust 1.95, now declared by tooling. An independent second handle is rejected; closing the first permits acquisition. Only Linux was exercised.

## Verification

- The red phase was attempted first with regressions covering the original behavior, but Cargo initially could not unpack its read-only registry, then compilation hit an incomplete concurrent native refactor. Those attempts did not execute a red assertion. Baseline failures were established through the original code and actual Library identity contract; do not describe the blocked runs as observed failing tests.
- Writable cache: `CARGO_HOME=/tmp/daydream-cargo-home`, `CARGO_BUILD_JOBS=2`.
- Targeted privacy run: 8 passed (seven new regressions and one existing privacy-safe-error test), `/tmp/daydream-processing-target.log`.
- `cargo test --offline --locked --manifest-path apps/desktop/src-tauri/Cargo.toml --lib ai:: -- --test-threads=2`: 71 passed, `/tmp/daydream-processing-ai.log`.
- Additional filters against that freshly compiled executable: Library 24 passed, retention 3 passed, transcription 17 passed, recording leases 1 passed, deletion 2 passed: 118 ownership tests total. Transcription's local HTTP tests initially failed because sandbox loopback binding was denied; the automatically approved escalated executable rerun passed all 17. Evidence: `/tmp/daydream-processing-owners.log`, `/tmp/daydream-processing-transcription.log`, `/tmp/daydream-processing-boundaries.log`.
- Scoped rustfmt and `git diff --check` passed. A subsequent dispatcher optimization avoids inspecting every queued chunk on every claim; final shared Rust validation must compile that change.

Durable scenarios cover synthetic audio/video redaction over UUID-owned results, reversible unredaction, remote VLM request bodies excluding hidden prior text, changed-redaction dispatch, open/closed protection barriers, pause/cancel crash recovery, late cancelled/superseded transcript persistence, late cancelled visual persistence, crash artifact deletion, active retention deferral and later cleanup, mutation completion ordering, and app-data lock release.

No real models, hosted providers, capture permissions, or manual desktop UI were exercised. The VLM payload assertion uses the actual request builder and persisted/query-visible context; it does not send content to a hosted provider. A request already sent remotely cannot be retracted. Metadata redactions do not cut MKV media bytes. Long native extraction calls may delay mutation completion until work drains.

## Local REST handler drain follow-up

Independent cross-review found that cancelling the client REST future could return before the separate local handler removed its WAV copy. Added request receipts retaining the existing processing lease, pending-request revocation, cleanup acknowledgement, and shared job/request scratch ownership across the Whisper subprocess. Cancellation/timeout/transport failures terminate and wait for the child before outer cleanup. No mutex is held across model calls, network requests or drain awaits.

The delayed-handler regression was observed failing for the intended reason before receipt activation: `client returned before local handler acknowledged cancellation` (`/tmp/daydream-local-drain-red.log`). After the fix, the full transcription filter passed 22 tests (`/tmp/daydream-local-drain-green.log`), including five new real-loopback regressions: drain acknowledgement, successful deletion waiting for WAV cleanup, aborted-client handler ownership, rejected unconsumed request releasing its lease, and revoked pending request refusing delayed copy creation. The runner boundary is mocked; the HTTP adapter/server, receipt state and filesystem cleanup are real.

The same compiled batch includes the dispatcher optimization. Native owns the final shared Rust gate. Handler panic cleanup follows Rust unwinding/drop semantics but was not separately injected in these five tests; forcefully terminating the whole application is covered by startup scratch cleanup rather than in-process acknowledgement.

## Final failed-parent responsiveness delta

Native reported its full 349-test suite green before this bounded finishing change. The cancellation poll now treats every parent state except `running` as a reason to cancel an old request. The added real-loopback regression marks the durable parent failed while a mocked model is active, invokes the actual cancellation poll, and proves prompt request cleanup plus processing-lease release. It first failed with `failed parent left active inference holding its processing lease` (`/tmp/daydream-failed-parent-red-run.log`), then passed.

The final rebuilt executable passed AI 71 and transcription 23 tests, 94 targeted checks total, including the new scenario (`/tmp/daydream-final-processing-tests.log`). The full suite was not repeated for this bounded delta. Four processing-owned Clippy findings were resolved without suppressions: visual extraction/batch request structs, typed frame-state updates, and `slice::from_ref` in a deletion test. Scoped rustfmt passed; native owns the final shared fmt/Clippy check after its separate warnings are fixed.
