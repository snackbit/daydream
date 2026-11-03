# Workflow testing

Use [validation commands](agent-validation.md) for scoped checks and final gates.
The [native runner README](../apps/desktop/scripts/native-e2e/README.md) owns
Linux prerequisites, phase selection, fixture isolation and driver setup.

## Test boundaries

Browser tests exercise React screens, routing, stores and typed clients with
synthetic Tauri responses. They do not prove SQLite, native playback or devices.

Rust tests exercise commands, storage, migrations, media writing, processing and
privacy with temporary databases and synthetic media. Model tests exercise the
managed worker with pinned models and synthetic passages.

Native UI journeys run the actual Tauri/WebKit app, commands and persistence.
The capture adapter supplies synthetic pixels; ambient microphones and screens
stay off. Private app-data directories isolate runs and survive test restarts.
Missing prerequisites fail the run.

## Workflows to protect

| Area | Required outcomes |
| --- | --- |
| Capture | Source setup persists; start/pause/resume/stop respect pending ownership; failures preserve recoverable files and visible controls. |
| Studio | Playback and source navigation retain timing; redactions hide results immediately; stale requests cannot overwrite a newer window; failed refreshes retain readable sources. |
| Processing | Independent lanes publish progress and results; pause/cancel drain owners; retry preserves successful work and exposes failed sections. |
| Settings | Drafts survive failed saves; Cancel discards edits; remote consent identifies destination and content; model transfer supports cancellation and recovery. |
| Storage | Cleanup requires confirmation, drains owners, preserves excluded data and resumes partial failure; migrations preserve populated databases. |
| Recall | Search revalidates eligibility; citations open the source; edits survive navigation; privacy changes withhold affected generated text. |
| Shell | Navigation, appearance, recording controls, notifications and diagnostics remain usable with the keyboard and at narrow widths. |

Test ordinary outcomes and reachable failures: duplicate actions while pending,
late responses, navigation during work, restart, unavailable tools, privacy
changes and retry. Use both themes and representative desktop/narrow viewports
for changed interfaces. Assert visible outcomes and durable effects.

## Owning tests

Frontend workflow tests live beside features under
[desktop source](../apps/desktop/src/features/). Native scenarios live beside
their Rust owners under [native source](../apps/desktop/src-tauri/src/).
The [native journeys](../apps/desktop/scripts/native-e2e/journeys.mjs) compose
running-app capture, Studio, settings/restart and keyboard scenarios.

## Manual checks

Automated fixtures do not establish real portal permission behavior, microphone
accuracy, compositor support, hosted-provider reliability or installer correctness.
Check those boundaries on the target system when they change. Never attach private
recordings, OCR, transcripts, prompts or unredacted logs to test artifacts.

Record current results and limitations in the task or change description.
Do not maintain cumulative pass counts or local temporary log paths here.
