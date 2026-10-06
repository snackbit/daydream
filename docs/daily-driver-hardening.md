# Daily-driver hardening

Implemented October 2026 as one ad hoc change across capture, processing, storage
and the desktop UI. This report describes implemented behavior; actual microphone,
Wayland and packaged-app checks remain operator work.

## Processing and recovery

Audio transcription, literal OCR, screen observations and activity continue
independently after ordinary section failures. Jobs finish with
`completed-with-errors` when some sections remain incomplete. Successful OCR
frames, observations and speech ranges are committed incrementally and retained
on retry. Current privacy eligibility is checked before work and persistence.

Transient provider failures receive bounded retries with backoff. Malformed
responses receive one automatic retry; configuration and damaged-resource errors
require intervention. There is no automatic provider or CPU fallback. Errors
expose stable codes and readable explanations, with links to relevant Settings.
Technical details remain available without including provider response bodies.

The Processing screen shows active work and history, stage progress, elapsed time,
estimated remaining time when enough completed work exists, retry timing and
individual failed sections. Live jobs have an open-ended recording input and do
not promise a completion ETA. Users can pause, resume, cancel, retry failed units
with saved or current settings, and explicitly reprocess both audio tracks.
Media preparation and model downloads also appear in the dashboard.

Blocking extraction, detector and native work runs outside async request handlers.
Subprocesses have cancellation, output bounds and timeouts. Model/provider
concurrency is bounded, duplicate OCR requests share exact-image recognition, and
idle live jobs release scheduler capacity.

## Capture and shell

New installations default to processing during recording in Background mode.
Settings offers During recording, After stop and Manual, plus Background/Fast
resource use. Saved processing configuration belongs to each job; targeted retries
can explicitly adopt current settings.

Capture rotates approximately 30-second physical Matroska fragments while keeping
the capture session alive. Only finalized, classified, eligible sections enter
processing. The unfinished tail waits for neighboring finalized audio context.
Interrupted capture masks any unclassified tail conservatively. Protection-save
failure retains unresolved spans and visibly blocks unsafe processing.

The shell footer exposes Pause, Resume and Stop outside Capture, alongside recorded
duration excluding pauses and current file size. Controls disable during lifecycle
transitions. Successful and failed finalization remain visible through native
status.

## Speech detection

Both audio tracks run the bundled offline Silero v6.2.0 detector before Whisper.
The model checksum, pinned source and MIT license live in
`apps/desktop/src-tauri/resources/vad`. Detection runs on CPU with owned native
resources and cancellation checks. Internal defaults are probability 0.5,
minimum speech 250 ms, 400 ms of non-speech to close an utterance, and 300/150 ms
of leading/trailing padding. The existing minimum speech setting is honored.

VAD windows intersect the optional loudness gate; loudness gap merging cannot
reconnect distinct VAD utterances. No surviving speech means no Whisper or remote
transcription call. Detector failure is a retryable section error. Eligible audio
context crosses contiguous chunk/fragment boundaries, while pauses and redactions
prevent context crossing. Transcript segments and words are validated against the
submitted window, with ownership preventing duplicate boundary words.

Saved audio remains unchanged. Existing transcripts stay unchanged until explicitly
reprocessed. Reprocess audio tracks replaces audio-derived results using current
settings and retains other AI sections and original media.

## Studio

Focus changes, OCR and screen observations have persistent description rows beneath
their point markers. Descriptions use available space up to the next point or
source/privacy boundary. Mouse and keyboard bands remain compact.

Waveforms fill the audio clip instead of inheriting button-icon dimensions.
Adjacent redaction lanes share one caption and mask control; horizontal pointer or
keyboard resizing saves atomically and restores the saved range on failure.
Opaque lane backings remain in place. Comma and period pause playback and move to
actual saved frame timestamps, rather than relying on current capture settings.

## Compatibility and validation

Public transcription settings and existing commands remain available. Additive
commands, events and optional settings support dashboard work, source retries,
audio reprocessing, frame navigation, recording statistics and redaction resizing.
Shared TypeScript consumers validate new responses. Explicit schema migrations
21–23 preserve populated jobs and child rows, introduce partial completion,
runtime/checkpoints, unit retry selection and live classification metadata. The
legacy v1 migration also handles foreign-key-disabled table rebuilds.

Automated verification includes real bundled detector tests for typing, breath-like
noise, quiet speech, short replies, speech mixed with typing and chunk boundaries;
timestamp bounds, skipped provider calls, cancellation and model failures;
privacy/redaction recovery, retries and migrations; real media frame navigation;
synthetic multi-fragment capture; dashboard recovery, waveform geometry, studio
selection/redactions/frame stepping and global recorder controls.

Validation commands and final counts are recorded in
[refactoring tasks](refactoring-tasks.md). Browser workflows use synthetic IPC
fixtures. Detector fixtures use synthetic audio, not captured conversations.

## Operator checks still required

- Record actual microphone typing, breathing, quiet speech and short replies on
  both tracks. Check false positives, missed speech and word boundaries before
  adjusting sensitivity; synthetic fixtures do not establish microphone accuracy.
- Exercise real Wayland portal/PipeWire capture, permissions, pause/resume/stop,
  protection rules and a long recording with live processing.
- Smoke-test the packaged app with its bundled VAD resource and host media plugins.
  Resource mapping and checksum are verified in source; a development build is
  not a packaged-install test.
- Exercise configured remote providers under real rate limits and latency. ETA is
  an estimate; fixed retry backoff does not promise provider-specific scheduling.

Reversible redactions remain metadata masks; they do not physically erase original
media. Recording deletion retains the existing physical-deletion contract.
