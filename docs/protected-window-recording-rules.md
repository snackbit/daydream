# Protected Window Recording Rules

Date: 2026-06-13

## Summary

Add recording settings that pause recording or create automatic redactions when
protected windows are focused. Rules apply only to future recordings. Users can
set a global default action and override it per rule.

Default timing:

- Focus dwell: `2s`
- Protected span merge gap: `5s`
- Manual pause wins over automatic resume.

## Decisions

- Support both automatic pause and automatic redaction.
- Settings include a global default behavior: `pause` or `redact`.
- Each rule can inherit the global default or explicitly use `pause` or
  `redact`.
- Rule matching uses app/process identity plus optional window-title matching.
- Users can add rules manually or through a picker of currently open
  windows/processes.
- Automatic redaction targets all tracks by default: video, desktop audio, and
  microphone.
- Rule changes are future-only. They do not retroactively edit existing
  recordings.
- V1 automatic redaction is metadata plus AI-skip. Physical media byte cutting
  is out of scope for this feature.

## Key Changes

### Shared Settings And Types

Add shared settings and Rust DTOs under `record.windowProtection`:

- `enabled: boolean`
- `defaultAction: 'pause' | 'redact'`
- `dwellMs: number`
- `mergeGapMs: number`
- `rules: ProtectedWindowRule[]`

Rule shape:

- `id: string`
- `enabled: boolean`
- `label: string`
- `action: 'inherit' | 'pause' | 'redact'`
- `appIdentity?: string | null`
- `processName?: string | null`
- `titleMatcher?: WindowTitleMatcher | null`

Title matcher shape:

- `kind: 'contains' | 'regex'`
- `value: string`
- `caseSensitive: boolean`

Add validation limits for dwell, merge, matcher length, and rule count.

### Settings UI

Add a focused protected-window section in the Record settings tab:

- Global enable toggle.
- Radio-style default behavior: pause or redact.
- Numeric dwell and merge controls.
- Rule list with enable, delete, label, process/app matcher, title matcher, and
  action override.
- Picker dialog populated from currently open windows/processes.
- Unavailable states for unsupported platforms, unsupported desktop sessions,
  permission denial, and provider failure.

Use existing shadcn components from `apps/desktop/src/components/ui` where
available.

### Tauri Commands

Add typed commands:

- `listProtectableWindows`
- `getWindowProtectionStatus`

Returned errors should be typed, not string-only guesses:

- `unsupported_platform`
- `session_unsupported`
- `permission_denied`
- `provider_unavailable`

Do not log raw window titles, window inventories, OCR text, transcript text, or
private payloads.

### Platform Boundary

Add a native active-window provider behind a typed interface.

Linux first:

- X11: use EWMH `_NET_ACTIVE_WINDOW`, `_NET_CLIENT_LIST`, `_NET_WM_NAME`,
  `_NET_WM_PID`, and class/process metadata where available.
- Wayland: use `wlr-foreign-toplevel-management` when the compositor supports
  it.
- Return typed unavailable errors when the session does not expose active
  window metadata.

Windows and macOS:

- Add typed stubs for this task.
- Do not fake success.
- Keep the boundary ready for real implementations.

Do not use XDG ScreenCast as the active-window inventory API. It is a capture
source API, not a focused-window metadata API.

## Recording Behavior

Window protection starts with a recording and continues while the recording is
paused, so it can auto-resume after a protected window is no longer active.

Matching:

- Evaluate rules in memory against current focused-window metadata.
- Prefer app/process identity over title matching.
- Treat titles as sensitive metadata.
- Do not persist full open-window inventories by default.

Automatic pause:

- If a matching rule remains focused for `dwellMs`, begin recorder pause.
- The initial dwell interval that was recorded is materialized as an all-track
  redaction.
- Resume only after no protected match exists for `mergeGapMs`.
- Never auto-resume a recording that the user manually paused.

Automatic redaction:

- Keep recording.
- Open a protected span after dwell.
- Close the span after no protected match exists for `mergeGapMs`.
- Materialize closed spans as all-track redactions with a reason such as
  `Protected window: <rule label>`.

Add durable `recording_protection_spans` storage so crash/recovery can close or
materialize automatic spans before AI processing starts. Existing manual
redactions stay unchanged.

## Processing And Privacy

Current AI processing blocks redacted recordings. This feature must include
redaction-aware processing chunk planning or automatic redactions would block
auto-processing.

Required processing changes:

- Remove the broad "redactions unsupported" AI-processing block.
- Split or omit audio, visual, OCR, VLM, transcript, and activity chunks that
  overlap redacted spans.
- For activity processing, any redaction overlapping the time range excludes
  that activity slice because activity can reveal private use.
- Ensure automatic redactions are materialized before auto-processing is
  enqueued after recording stop.
- Keep query-time redaction filtering in place for existing derived data.

V1 does not rewrite existing media files. Physical media cutting is a later
privacy-hardening task.

## Implementation Notes

- Store settings in the existing app settings key/value persistence layer.
- Keep React out of SQLite and native focus APIs; use typed Tauri commands.
- Keep route/screen files thin. Put feature UI under
  `apps/desktop/src/features/settings/`.
- Keep the active-window monitor separate from recorder lifecycle logic. The
  recorder should consume protection decisions, not OS-specific window data.
- Privacy-sensitive telemetry should report provider status and rule IDs only,
  not raw titles or process/window inventories.

## Test Plan

Rust tests:

- Settings defaults, validation, persistence, and reset behavior.
- Rule matching for process/app identity and title matcher combinations.
- Dwell timing and merge-gap grouping.
- Action inheritance and per-rule overrides.
- Manual pause precedence over automatic resume.
- Auto-pause lifecycle: pause, dwell redaction, merge-gap resume.
- Auto-redact lifecycle: span open, span close, all-track redaction creation.
- Crash/recovery behavior for open protection spans.
- Redaction-aware AI chunk planning and processing enqueue with redactions.
- Typed unavailable errors from platform providers.

Frontend tests:

- Record settings shows protected-window controls.
- User can enable protection, set default behavior, add/edit/delete a rule.
- User can add a rule from the open-window/process picker.
- User sees unsupported or permission-denied provider states.
- Validation errors block save and identify the broken setting.

Manual checks:

- Linux X11 active-window detection.
- Linux Wayland supported compositor path.
- Linux Wayland unsupported compositor state.
- Recording does not auto-resume after a user manual pause.

## Open Follow-Ups

- Implement real Windows and macOS providers.
- Add physical media cutting for redacted spans.
- Consider an explicit "apply to existing recordings" workflow once focus
  events are durable and safe to inspect.
- Add an audit view for automatic protection spans without exposing private
  window titles by default.
