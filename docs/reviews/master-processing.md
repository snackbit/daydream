# Master review: processing and privacy baseline

Reviewed clean main `6527f5fb14435f52f2bd880dde0e9635eaa50506`, entire-repository review partitioned to processing/storage/backend privacy. This records the original read-only findings; subsequent fixes do not rewrite this baseline.

| Priority | Category | Confirmed finding | Baseline evidence |
| --- | --- | --- | --- |
| High | Security | Library synthetic track IDs do not match UUID IDs used by transcript/visual visibility filters; normal redactions leave derived text exposed. | `ai/results/transcript.rs:50`, `ai/results/visual.rs:175`, `library/redactions.rs:20`; tests seed UUID targets at `ai/tests.rs:699`. |
| High | Security | Queued, paused, retried and active processing does not recheck changed redactions, permitting processing/upload of hidden media. | `library/commands.rs:149`, `ai/processing/worker/dispatch.rs:30`. |
| High | Security | VLM context reads raw OCR/observations, including newly redacted prior text. | `ai/processing/visual/mod.rs:662`, `:724`. |
| Medium | Logical Flaws & Bugs | Restart during Pause/Cancel leaves running chunks stranded because startup repairs only running parent jobs. | `ai/processing/worker/scheduler.rs:8`, `worker/actions.rs:65`. |
| Medium | Security | Recording deletion does not remove extracted processing files left after abrupt exits. | `ai/processing/temp_files.rs:22`, `library/mod.rs:64`, `storage_limits.rs:222`. |

Native integration: enqueue/dispatch has no protection-span guard after stop/recovery materialization fails. Native reviewer owns the lifecycle/materialization finding; processing must fail closed while open or closed-unmaterialized spans remain.

Authoritative sources: AGENTS.md, README implemented behavior/privacy statements, M2 processing foundation, and protected-window-recording-rules.md:164. M2.1 defers physical cuts/derived deletion; M3 remains roadmap. Metadata redactions must remain reversible.

Inspected: planning, dispatch, lifecycle recovery, transcription adapters/local HTTP, OCR/VLM extraction/persistence/context, result visibility, Library redaction/deletion/debug/playback, retention, settings/secrets, notifications, schema and shared contracts. Model management, migration upgrade permutations, waveform math and HTTP concurrency received limited inspection.

Checks: focused source/fixture inspection and clean Git status. No edits, builds, tests, migrations, installs or provider calls during review. Existing Rust binaries were skipped because their correspondence to HEAD was unknown. No real capture/manual UI validation. Frontend reviewer separately found stale derived-result display after redaction.
