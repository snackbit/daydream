# Recorder media shutdown validation

The media process owns GStreamer pipelines and Pulse streams. The application
owns capture, crop/protection decisions and SQLite. The private control socket
carries configuration and small lifecycle messages; a separate socket carries
validated binary frame headers and owned pixel buffers. A configuration must fit
the 512 MiB transport allowance and cannot exceed 5 fps. This admits dual 4K and
dual 5120×1440 outputs at native resolution with the existing conservative
seven-frame estimate and socket reserve. The allowance bounds the transport
estimate, not total application memory. Pending capture frames
replace the unsent frame for their source, and appsrc has one queued buffer.

Stop and pause use one monotonic 20-second deadline from the user request. The
worker has at most 15 seconds for normal completion, then two seconds for flush
and three for kill/reap. A failed stop clears finalization and preserves the
files. If cleanup cannot establish quiescence, status reports `quarantined` and
capture/recovery/maintenance remain blocked. Status failure can be read without
waiting for a blocked runtime owner. Late completion cannot publish success or
start automatic processing.

The observed user log established a desktop-source EOS timeout followed by
unbounded pipeline teardown. It did not identify the native lock owner. Upstream
[GstBaseSrc EOS handling](https://github.com/GStreamer/gstreamer/blob/main/subprojects/gstreamer/libs/gst/base/gstbasesrc.c#L1816)
holds the element state lock while changing flushing state and acquiring the
source streaming lock. A blocked EOS sender can therefore prevent a subsequent
state change to Null. The
[Pulse source reset path](https://github.com/GStreamer/gstreamer/blob/main/subprojects/gst-plugins-good/ext/pulse/pulsesrc.c#L1539)
also takes its threaded-mainloop lock and waits for the server's flush operation.
The exact Pulse wait in the original failure remains unconfirmed.

An unmodified synthetic 5120×1440, 0.2 fps, 14-second recording with two live
`audiotestsrc` branches allowed the old desktop-EOS-first sequence to return.
That run did **not** reproduce the original Pulse timeout. Its diagnostic test
records the actual result. A separate fixture deliberately holds a real
GStreamer source streaming lock and checks the EOS timeout, blocked Null
transition and bounded process termination. This proves the lock failure path;
it does not establish the original device/server trigger. The host provides
GStreamer 1.28.7 and `pipewire-pulse`, but no standalone `pulseaudio` daemon was
available for an isolated Pulse-server fixture. No ambient service or recording
device was changed or used.

Focused native coverage also checks successful wide recording, maximum-fps
transport timing, malformed/truncated frame headers, unexpected transport EOF,
fragment identity and terminal reference-video EOS, copy-remux coverage,
creator-thread lifetime, Linux parent-death kill/reap, and same/swapped CLOEXEC
descriptor handoff. Playback tests cancel blocked HTTP readers and kill/reap
active subprocesses before reporting drained resources. All media fixtures are
synthetic. Final gate results are recorded in the feedback implementation ledger.
