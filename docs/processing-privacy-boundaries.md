# Processing privacy boundaries

Redactions are reversible metadata masks. They exclude content from processing
and derived results but do not erase the original MKV bytes. Recording deletion
removes media and derived artifacts.

## Eligibility and ownership

Library and processing use stable source identities such as `video-0`,
`desktop-audio-1` and `microphone-2`. Processing foreign keys remain UUIDs;
legacy UUID redaction targets remain readable.

Workers check current redactions before claiming work, extracting inputs,
uploading content and persisting results. Open or unmaterialized protection spans
block processing. See [window protection](protected-window-recording-rules.md).

Each recording has a read/write lease. Processing holds a read lease through
extraction, inference, persistence and temporary-file cleanup. Redaction and
explicit deletion cancel active work and wait for the exclusive mutation lease.
They do not hold the database mutex while waiting. A remote request already sent
cannot be retracted; the mutation waits for its owner to stop or finish.

A job may continue model work only while its durable parent is running. A failed,
paused, cancelled, completed, missing or unreadable parent cancels the old request.

## Files and recovery

An app-lifetime OS lock on `recording-data.lock` prevents two processes from
opening the same data directory. The OS releases it after a crash.

Retention defers recordings with active processing leases. Explicit deletion
drains owners before removing their artifacts and relational metadata. Startup
removes abandoned processing scratch files before workers start.

Prepared playback excludes tracks whose source range overlaps a redaction.
Sessions and in-flight requests hold file leases; superseded prepared files are
reclaimed after their final owner releases them. Changed privacy fingerprints
revoke old sessions. Source media remains available for reversible unhide.

## Local transcription

Internal transcription requests register receipts that share the processing
lease. Pending receipts are revoked on cancellation. Active handlers keep their
receipt until WAV copies and model children have stopped and cleanup completes.
Dropping the client cancels the receipt without releasing the handler's ownership.

WAV copies live in job/request scratch directories. Nested Whisper requests reuse
that directory. Cancellation, timeout and transport failure terminate and join
the subprocess before cleanup. A delayed runner keeps deletion pending until it
drains. These local requests do not change remote-provider consent.
