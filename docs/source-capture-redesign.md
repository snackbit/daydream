# Source capture and Settings redesign

Implemented on 2026-10-06. This extends the [daily-driver changes](daily-driver-hardening.md).

## Capture and Studio

Capture now lists monitor sources beside a preview and source output controls.
Each source has a full-source or numeric region crop and an output scale. Crop
coordinates use physical source pixels. Crop applies before resizing, and playback,
OCR and observations use the saved output. Output dimensions are rounded down to
even pixels for encoding; upscaling is rejected.

For a 5120 × 1440 monitor, 50% produces 2560 × 720. To retain a native-size half,
choose a 2560 × 1440 region at X=0 or X=2560, Y=0, with 100% scale. Frame interval
and video bitrate are shared settings; the bitrate applies to each video source.
The source setup and those recording settings save in one transaction.

New recordings store one video stream per monitor in MKV, plus the enabled desktop
and microphone audio streams. Source IDs, output dimensions and stream indexes
remain stable across finalized fragments and pause/resume. Capture settings are
frozen until stop. A source-size change fails capture visibly instead of silently
changing the recording geometry. Finalized fragments remain available.

Studio defaults to a selected-source player. The source picker changes playback
and frame stepping; Grid shows synchronized video players with one audio playback
owner. Redactions and derived results retain their source identity. Hiding Screen A
does not hide Screen B. OCR and observation processing plan work independently for
each saved video source, applying that source's redactions before extraction.

Existing recordings retain their composite media and legacy stream mapping.
Independent-source processing requires a source ID. Jobs previously queued without
one fail before extraction and need a new OCR reprocess; they cannot silently use
the first screen. Legacy composite recordings retain their original fallback.
Existing canvas setups require one review/save: crops are retained and output
scale resets to 100%. Canvas placement no longer controls new recordings.

## Settings and Vision

The first-save failure came from checking saved credential readiness before the
new key had been saved. Structural validation now accepts incomplete profiles;
settings and encrypted credential drafts commit together. Failure rolls back both.
An incomplete Vision setup displays a warning and affected processing tasks fail
visibly. Other settings and independent processing remain usable.

Authentication is explicitly API key or No authentication. API key remains the
default for existing profiles. Anonymous requests omit Authorization. Credential
replacement removes unreferenced encrypted values while pinned job configurations
retain the keys needed for their retries.

Test Vision uses the unsaved form and generates a local PNG spelling TEST. It sends
no captured frames, saves no settings or results, and reports each enabled OCR or
observation check separately. Each check has a 20-second timeout. OCR must recognize
the generated text; transport success alone is insufficient. Editing the form hides
results from an older configuration. Provider bodies and key drafts are not logged.

OCR selection combines Off, local Tesseract and Vision. The endpoint form is shared
with observations; data disclosure and per-source request estimates are collapsed
under Data and usage. Settings fills the main pane. Header heights match and the
active tab uses a single bottom underline.

## Compatibility and verification

Schema 24 adds source recording and fragment mapping tables plus the setup mode;
old rows use the legacy mode. Existing public commands remain available. Additive
commands are `getCaptureSetup`, `saveCaptureSetup` and `testVisionProvider`.
Library/playback metadata adds optional video-source lists, with runtime decoding
at the frontend boundary. No user database was reset or changed during verification.

The writer uses GStreamer's splitmux auxiliary video pads and explicitly maps
Matroska stream indexes. Multi-source capture checks this capability before writing.
The tested host uses GStreamer 1.28.7. Missing auxiliary-pad support produces an
actionable recorder error. The direct gstreamer-video dependency matches the existing
0.24 family; the Cargo-generated lockfile records it.

Automated checks cover:

- First-save and incomplete profiles, atomic credential rollback and anonymous auth.
- Synthetic Vision requests, provider failure, unsaved form testing and stale results.
- Crop/scale workflows including the 5120-pixel monitor examples and invalid crops.
- Independent source pixels, metadata, source-specific redaction ranges and deletion.
- A real writer with two video sizes over a synthetic 70-second timeline, producing
  at least three independently playable fragments with stable stream indexes.
- Source selection, grid masking, actual-frame navigation and existing Studio workflows.
- Capture and Settings at 1440 and 420 pixels in light and dark themes, including
  visible controls, no horizontal overflow and Settings content alignment.

Final command results are recorded in [refactoring tasks](refactoring-tasks.md).
Screenshots use synthetic fixtures and mocked IPC; they do not establish native
device behavior.

## Operator checks still required

No ambient screen or microphone recording was started. Real Wayland portal,
PipeWire, WebKit and packaged-app checks remain pending:

1. Select two monitors, review their physical dimensions, save, record, pause,
   resume and stop. Confirm separate playable streams and both audio tracks.
2. On a 5120 × 1440 monitor, check 50% output and a positioned 2560 × 1440 crop.
   Confirm preview, saved playback, OCR and frame stepping agree.
3. Disconnect or resize one monitor while recording. Confirm a visible failure,
   preservation of completed fragments and a usable restart after review.
4. Redact one monitor in Studio and confirm the other remains visible and eligible
   for processing. Check selected-source and grid playback in the native WebKit app.
5. Test an actual Vision endpoint with new credentials and with explicit anonymous
   auth. Confirm saving the engine/profile/key in one operation works.
6. Run the previously documented microphone checks for typing, breathing, quiet
   speech and short replies. Synthetic VAD fixtures do not prove every microphone.

Hardware encoding and GPU composition were not added. Crop/scale transforms remain
on the CPU, so real multi-monitor load and packaged runtime compatibility still need
measurement on the target machine.
