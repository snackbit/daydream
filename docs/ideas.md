# Ideas

This file is for interesting product or technical ideas that are worth
revisiting later, but are not current implementation priorities.

## Desktop Metadata Ideas

### Window metadata capture hardening

Activity capture now records focused-window changes and deduplicated open-window
inventory snapshots where the platform provider supports window metadata.
Future hardening should add macOS and Windows providers, richer review UI for
open-window snapshots, and physical deletion of any derived window metadata when
redacted spans are cut.

Window titles and open-window inventories are private content. They must not be
logged or sent to remote services by default, and Activity redactions must hide
the corresponding focus events and inventory snapshots.

## Audio Capture Ideas

### Optional acoustic echo cancellation for microphone transcription

Desktop audio can leak into the microphone path, especially when speakers are
active or the microphone is sensitive. That can cause the desktop audio to be
transcribed on the microphone track too, which creates false mic evidence.

This is worth investigating later, but it is not a priority right now.

Potential options:

- GStreamer `webrtcdsp` plus `webrtcechoprobe`. This is the best fit for the
  current Linux recorder because the media writer already uses GStreamer, and
  WebRTC AEC uses the desktop audio as a far-end reference.
- PipeWire echo-cancel virtual source. This may be useful as a detected or
  selectable system source on Linux, but it is less portable and sits outside
  Daydream's recorder pipeline.
- RNNoise or other noise suppression. This can help with background noise, but
  it is not full echo cancellation because it does not use the clean desktop
  audio reference.

The main thing to avoid is treating plain ANC/noise suppression as the primary
solution. The real problem is acoustic echo from a known reference stream, so
AEC is the better investigation path.
