# Studio structure and verification

Studio now keeps its recording composition component at the root. Playback,
timeline, Inspector, navigation, media, and result-window modules live with their
owners. Tests move with those owners. The root has one TypeScript file instead of
72; every Studio folder has at most nine direct TypeScript files, including tests.
Production modules stay below 400 physical lines. Recording composition and the
playback controller are the largest, at 389 lines each.

The timeline splits into a 60-line composition component, 326-line controller,
249-line layout renderer, and 90-line controls renderer. Pure layout functions keep
viewport and sidebar state with their original React owners. The sidebar shell is
128 lines; selection, tab transitions, and Details rendering have separate owners.
Playback segment completion, gaps,
calendar-slice boundaries, and completion deduplication live in a 137-line hook.
Timeline store, selection math, and geometry retain all 16 original unit scenarios
in separate suites. Export inventory checks found no lost named exports.

Component/hook contracts, store identity, routes, IPC, privacy eligibility, and
storage remain compatible. Consumers use the new internal import paths. The
sortable lane wrappers now forward their full typed props, fixing an existing
omission of Day Studio's transcript reader, revision, and scoped loading/error
state. A browser regression failed before the fix because the day reader received
no request. The native day tooltip subsequently displayed its eligible excerpts.

## JavaScript evidence

`COREPACK_HOME=/tmp/corepack corepack yarn check` passes lint, workspace
typechecks, 1,113 desktop tests, nine shared tests, and one FFmpeg test on the final
tree. The gate log is `/tmp/daydream-studio-js-complete.log`. Lint retains
the existing TanStack Virtual compatibility warning. The library workflow run
passes 357 scenarios after the layout adjustment. Focused additions cover source selection, locked recordings,
stale clips, waveform failure, follow restoration, early Play, native completion
events, gap timers, calendar slices, and the sortable day-reader handoff. Additional
replay and transcript failure/recovery scenarios also pass.

The final source-mapped V8 library run covers 191/197 split-owner branch outcomes
(96.95%). Each of the nine owners with instrumentable branches exceeds 90%,
ranging from 93.10% to 100%; the composition facade has no instrumentable branches.
The report is `/tmp/daydream-studio-final-coverage/` and the scoped command/results
are in `/tmp/daydream-studio-final-workflows.log`. A controlled sensitivity run removed
the completion guard, observed the duplicate media revision assertion fail, and
restored the guard. Its log is `/tmp/daydream-studio-dedup-sensitivity.log`.

Light/desktop and dark/narrow screenshots were inspected. Browser tests replace
native IPC, subscriptions, and selected media operations at their external
boundaries. Timer tests exercise the real resume-timer hook with a controlled
clock. These checks do not establish physical capture or packaged device behavior.

## Native acceptance still open

Fresh frontend/native builds pass. The initial isolated GTK/WebKit journey passes
real media advancement, pause, playback speed, Studio tabs, synthetic capture,
settings persistence, and source navigation. Artifacts include
`/tmp/daydream-native-e2e.skfN4I/initial-report.json`.

The dense eight-hour Studio journey completes durable waveform preparation and
loads real day-scoped transcript previews, then times out while trying to reveal
individual transcript buttons after zoom. The retained 10-second minimum window
can leave the fixture's 400 ms utterances grouped in the narrow timeline. The
native keyboard journey also times out at its empty-search assertion outside
Studio. Neither failure is credited as a pass. A comparison using the pre-split
timeline, sidebar, and playback compositions reproduced the transcript zoom
failure on the same fixture. That comparison measured control/zoom/pan p95 at
22/99/85 ms. Refactored runs remained above the 100 ms zoom/pan budget: the latest
layout-renderer run measured 25/141/198 ms. The comparison restored the refactored
files exactly. Keeping pure renderers preserves the original React component tree
and state ownership, but does not establish timing conformance. Native performance
acceptance therefore remains open. Logs include
`/tmp/daydream-studio-native-comparison.log`,
`/tmp/daydream-studio-native-flat-render.log`, and
`/tmp/daydream-studio-native-keyboard.log`; synthetic artifacts are under
`/tmp/daydream-native-e2e.ez16P9/`.

The Studio fixture upsert now keeps an existing setting's update timestamp at
least as late as its creation timestamp. This repairs fixture setup against the
existing SQLite constraint and changes no production storage policy.

Import-only consumers retain existing structure debt: Day Studio is 485 lines,
its folder has 22 direct TypeScript files, and screens has 23. This cleanup adds
no files or behaviors there. No new size/density budgets, dependencies, migrations,
commits, or subagents were introduced.
