# Protected-window recording

Window protection can pause recording or mask all tracks when a protected window
has focus. Users configure rules in Recording settings, either manually or through
the open-window picker. Rules affect future capture, not existing recordings.

## Rules and timing

Settings live under `record.windowProtection`: enablement, default action,
dwell time, merge gap and rules. Each rule can inherit the default action or
select pause/redact. Matchers use app/process identity and optional contains or
regular-expression title matching.

The default dwell is two seconds; the merge gap is five seconds. A pause rule
masks the recorded dwell interval before pausing. A redact rule creates an
all-track metadata mask. Neither action physically cuts media.

Switching between protected rules keeps one continuous protected span. Pause
takes precedence for the remainder of that span. The span retains its initial
rule attribution and closes only after reliable metadata reports no match for
the merge gap.

## User control and failures

The backend owns pause authority. Protection continues monitoring while paused.
Automatic resume applies only to the automatic pause that created it. Manual
pause claims that authority; manual resume revokes it. A newer pause invalidates
an older pending resume, including while a segment is being prepared.

Unknown focus is not a safe nonmatch. If metadata becomes unavailable during
protected capture, the backend stops recording with a typed error. Protection
persistence failure also stops capture and blocks processing. Span closure and
redaction materialization share a SQLite transaction. Recovery must resolve
unfinished protection before processing resumes.

Capture workers and the media writer are supervised. Their failure stops the
remaining resources and publishes an error rather than leaving recording active.

## Platform boundary

Linux X11 uses EWMH focused-window and inventory metadata. Supported Wayland
sessions use wlr foreign-toplevel metadata or the KDE KWin session D-Bus bridge.
Other sessions report an unavailable provider. Windows and macOS providers are
not implemented. ScreenCast source selection is separate from window metadata.

`listProtectableWindows` and `getWindowProtectionStatus` expose typed availability
states. Titles and inventories are private content and must not enter normal logs.
Activity capture has its own metadata settings.

## Verification

Regressions cover rule matching, dwell/merge timing, continuous matches, manual
pause precedence, stale resumes, missing metadata, failed persistence, recovery
and processing exclusion. Picker workflows cover save failures and unavailable
providers. Real compositor permissions and capture require operator checks.

See [processing privacy](processing-privacy-boundaries.md) and the
[validation commands](agent-validation.md).
