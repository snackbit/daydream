# Feedback upgrade implementation

Approved scope: recording finalization, model transfers, shared controls,
maintenance, and processing grouped by recording. Keep the existing warm design,
English product copy, Tailwind tokens, and shadcn components.

- Isolate the GStreamer writer in the app executable before Tauri initialization.
  Drain video before concurrent audio EOS. Bound stop to 20 seconds, reap the
  child, and quarantine resources that cannot be confirmed stopped. Preserve
  originals; explicit recovery creates a separate recording with privacy and
  timestamp provenance. Recorder status has monotonic revisions and truthful
  pausing/stopping states.
- Append migration 26 for embedding assets and durable paused transfers. Admit
  persistence and notifications atomically before reserving the manager slot.
  Pause closes and flushes the partial file before acknowledging completion;
  resume validates HTTP ranges. Shared transfer rows expose progress, current
  throughput and defensible ETA, with persistent local errors and actions.
- Use aligned dropdowns, shared disclosure controls, and a searchable window
  picker with local application icons and left-aligned identity details. Keep
  actionable failures visible and translate technical codes into short guidance.
- Separate reset preferences, clear library, and factory reset. Preferences reset
  includes keys, scenes and theme, preserves library/models, and does not enforce
  retention. Library clearing includes user edits and derived data. Factory reset
  also removes models, partials, keys, restore tokens and logs, then restarts only
  after successful cleanup. Require two confirmations and typed confirmation for
  destructive actions. Append migration 27 for maintenance/recovery journals.
- Maintenance closes admission, cancels and awaits actual resource owners before
  deletion, revalidates impact, and preserves a retryable journal after failure.
  Never unlink the live database or claim OS permission revocation.
- Group processing before pagination by recording, with current transcription,
  OCR, observation and activity lanes. Report explicit skip/failure states,
  scoped retries and per-recording indexing. Keep shared model/global work below.

Owners: impl_media (writer/playback), impl_downloads (transfers/model lifecycle),
impl_maintenance (reset/recovery), design_shell (shared controls/picker),
feedback_processing (recording groups). Root owns integration, runtime admission,
status consumption, instruction updates and scheduled validation.

Use synthetic fixtures and targeted native/browser regressions. Schedule one
native build and one browser server at a time, then run relevant final gates once
the owners settle. Real desktop devices and personal data remain untouched.
