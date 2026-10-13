# Continuous Studio verification

The Studio rework replaces fragment cards with continuous source tracks, keeps
Review/Details state during processing, groups dense entries, and adds selection
zoom with five percent padding. Playback clock updates stay in small consumers
and an imperative playhead. Recording reads are bounded and cached; identical
Review rows keep their measurements and background publications yield to input.

Waveforms use durable local jobs and reusable five millisecond peak/RMS artifacts.
Viewport reads do not decode media. New visual jobs compare material changes with
up to three eligible earlier same-source frames. Matching Vision OCR and
observations share a request, with independent validation and section retries.
Existing jobs and public command names retain their compatibility paths.
The schema 36 migration adds waveform processing and visual sample lineage;
existing visual jobs default to their legacy protocol. Scoped read fields and
stream-seeking capability are additive IPC fields. No dependency versions changed.
Schema 37 adds a durable speech-language ledger and correction dependencies.
Auto mode detects every window; three substantial matching detections confirm a
language change. Corrective inference is limited to eight pending windows within
five minutes of the same job, track and configuration. Original published text
survives failed replacements. Earlier detection redactions withhold dependent
corrections, including search reads, and unhide restores eligible results.

October 13 follow-up: `listRecordingProcessingGroups` returned five native lanes,
but its frontend decoder still required four and rejected the complete page.
The decoder now requires each original AI lane exactly once and accepts the
additive waveform lane. Duplicate, missing and unknown lanes remain invalid;
legacy four-lane responses remain supported. The shared browser fixture now
matches the five-lane native producer. The red decoder check reproduced both
the rejected native response and erroneously accepted missing-lane responses.
The focused Processing gate passes 28 tests, including a routed screen refresh
through the actual client/decoder with only native IPC substituted. Decoder
coverage is recorded in `/tmp/daydream-processing-decode-coverage/` and the test
log is `/tmp/daydream-processing-decode-check.log`. Targeted lint and desktop
typechecking pass. Native code and storage were unchanged.

| Scenario | Evidence |
| --- | --- |
| Refresh while reviewing, repeat inspection, clearing selections, draft retention | Sidebar, recording Studio, and Review browser workflows. |
| Continuous media, zoomed-out counts, padded zoom, live-clock following | Assembled Studio browser workflow; grouping, projection, and window unit tests. |
| Source redaction and prior comparison-context redaction during pending reads | Day resource and recording Studio browser regressions; native eligibility tests. |
| Waveform admission with AI disabled, pause/cancel/restart, corruption, gaps, cleanup, deletion | Native waveform tests and actual FFmpeg fixtures. |
| Shared inference, malformed OCR with valid observations, same-source context, unchanged intervals | Actual native processing against an isolated local HTTP protocol fixture. |
| Weak acknowledgement followed by German speech; correction without pinning later detection | Actual native VAD, FFmpeg, processing, remote adapter and public transcript reads against a local HTTP fixture. |
| Review live highlighting, fast manual browsing, visible scrollbar and unique hover timestamp | Browser workflow with 700 entries and a 20-wheel burst, media clock and assembled Studio regressions. |
| Eight-hour native playback during updates and warm interaction | Optional native Studio workload: 960 fragments, four sources, 50,000 speech entries, 100,000 input intervals, and 50 interactions per category. |

Native format and all-target Clippy pass. Root lint and all workspace typechecks
pass; lint retains the existing React Compiler warning for TanStack Virtual.
The latest scoped desktop run passes 428 tests across 64 files. Changed executable
frontend branches measure 579/640 (90.47%) using source-mapped V8 coverage and
changed-line ranges; no difficult branches were excluded. Uncovered paths include
optional callback fallbacks, absent media geometry, superseded read ownership,
and incomplete processing states. Critical privacy and failure/retry outcomes
also have dedicated native and browser scenarios in the matrix above.

Before follow-up QA, all 819 desktop tests passed. The subsequent full desktop
run passed 819 of 820 tests; its sole failure was a synchronous ruler assertion
after moving measurement to the next animation frame. The owning test now awaits
the visible marker, and all eight ruler tests plus the scoped gate pass. The
follow-up live Review checks pass 13 scenarios; native language consensus, atomic
replacement and privacy checks pass 12 scenarios. Migration checks pass eight.
The settled full native gate passes 683 tests and three resource-staging tests;
four explicit benchmark/display-dependent tests remain ignored. Native evidence:
`/tmp/daydream-studio-native-settled.log`,
`/tmp/daydream-studio-clippy-last.log`, and `/tmp/daydream-language-last.log`.

Final JavaScript evidence: `/tmp/daydream-studio-latest-coverage.log`,
`/tmp/daydream-studio-latest-coverage/coverage-final.json`,
`/tmp/daydream-studio-changed-coverage.json`,
`/tmp/daydream-studio-lint-last.log`, and
`/tmp/daydream-studio-typecheck-last.log`. Root `yarn lint` and `yarn typecheck`,
native format/Clippy, and `sh -n apps/desktop/scripts/native-e2e/run.sh` were run
after the final edits. Shared tests (nine), FFmpeg resolver tests (one), and the
immutable install check passed earlier; dependency versions are unchanged.

The final native workload passed with control p95 24 ms, zoom p95 92 ms and pan
p95 82 ms (50 interactions each). It checks unchanged waveform files, retained
Review state during processing publication and no clock-triggered canvas painting.
These are input-to-second-animation-frame measurements against a 100 ms budget,
not a dedicated Review scrolling benchmark. The warm open took 693 ms; validating
the reused waveform artifacts took 1693 ms. The artifact's historical `coldWaveMs`
field names this latter measurement; this run does not measure cold preparation.
The isolated Broadway/WebKit runtime
uses synthetic media and a null audio sink. Browser tests replace Tauri IPC;
neither layer establishes live-provider model quality or real capture-device behavior.

The native language journey generated German speech with `espeak-ng`; the app
produced speech boundaries and transcripts. The HTTP provider was the only
transcription substitute. Requests were `auto, auto, auto, auto, de, auto`, and
the public transcript read contained the corrected output instead of the weak
English acknowledgement. This also exposed and fixed the remote adapter ignoring
the request's corrective language. Existing stored transcripts are not rewritten
on launch; a selected range can be reprocessed to apply the new behavior.

Native artifacts are under `/tmp/daydream-native-e2e.w0jKyk/`:
`studio-performance.json`, `language-processing.json`, `studio-inference.json`,
and both Studio theme screenshots. The final language/Studio runner log is
`/tmp/daydream-language-studio-native.log`. Synthetic secrets used an isolated
Secret Service daemon extracted under `/tmp/daydream-keyring-tools`; no host
packages or wallet settings changed. Validation-only package SHA256 values:
gnome-keyring 51.1-1 `8e6dc27e40ebadae6569098dff9fffcad4ca9f67c211951f80a17c01485b4de4`;
gcr 3.41.2-2.1 `d74c1c77f7e42bde847da1a3c2fccb18fccbf3645f6fb9b1f59d87c936d14cf1`.

The native speed journey selected 2x but measured 1x on the Linux streaming
WebKit path. Seekable Chromium playback-speed scenarios pass. This native runtime
limitation remains unresolved. Rust branch instrumentation is unavailable on this
host; native scenario and full-suite results do not substitute for branch coverage.

`TrackTimeline` remains the coordinate and selection coordinator (about 530 lines).
Media painting, lane rendering, grouping, window math, playback transport, and
inspection state have separate owners; the remaining JSX shares one viewport and
selection surface.

Use the commands in [agent validation](agent-validation.md) and the optional
workload described in the [native runner](../apps/desktop/scripts/native-e2e/README.md).
