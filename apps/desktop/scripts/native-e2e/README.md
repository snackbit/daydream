# Native UI journeys

These journeys run the actual custom-protocol Tauri executable, GTK/WebKit,
Rust commands and SQLite settings persistence. Playwright opens the local GTK
Broadway viewer and sends pointer events to native controls. The WebKit inspector
locates controls, observes rendered outcomes and reads public native commands;
it does not replace IPC, app stores or SQLite.
The driver briefly renders an inert origin marker to measure the WebView's actual
position inside GTK titlebar/shadow pixels, then removes it before stories and
screenshots. This avoids guessed pointer coordinates; the measured origin is
retained as a JSON artifact.
Keyboard uses the official WebKitWebDriver and Tauri/Wry's existing automation
environment flag. Standard W3C click/key actions reach the same native WebView
directly, with exact committed text and trusted key-event assertions. Broadway
remains its display and the initial/restart pointer transport. The two inspector
servers run in separate phases. Native event traces are retained; no characters
are retried or DOM input values assigned. The headless fixture selects GTK's
built-in simple input method and `C.UTF-8` locale; this standardizes the test
session and is not credited as the keyboard fix.

From the repository root, build and run the native journeys:

```sh
COREPACK_HOME=/tmp/corepack corepack yarn test:e2e:native
```

Prerequisites: Linux native build prerequisites, `dbus-run-session`, GTK3
`broadwayd`, a C compiler, installed Playwright Chromium, a WebKitWebDriver matching
the installed WebKitGTK protocol version, and permission to bind loopback ports
39188/39288/39289/39290. The driver is resolved from `DAYDREAM_WEBKIT_WEBDRIVER`
or `PATH`; it is required and never downloaded by the runner. Run one instance at
a time. The executable and
packaged embedding/VAD resources are produced by the debug build. The runner
builds fresh frontend assets and the custom-protocol executable by default and
does not download models. `--reuse-validated-build` skips builds for a coordinated
QA iteration only when that exact source and artifact state was already checked.
`PLAYWRIGHT_BROWSERS_PATH` may select an existing browser cache.
The default phase order is `initial restart keyboard`. For targeted diagnosis,
`DAYDREAM_E2E_PHASES=keyboard` selects only keyboard transport, and
`DAYDREAM_E2E_PHASES='initial restart'` selects capture/persistence. These are
partial runs; the final gate uses all default phases and reports each separately.

`DAYDREAM_E2E_LONG_PLAYBACK=1` extends the initial capture to 103 seconds. It
checks actual storage rollovers, full player duration and native seek/play after
30, 60 and 90 seconds. The regular journey keeps its short capture. Audio remains
off here; native synthetic-beep tests verify both audio streams, and browser
regressions verify waveform timing.
Play is pressed immediately after each seek, before waiting for media readiness.
`initial-seek-timings.json` records advancing-playback latency,
including pointer interaction, plus the pending thumbnail count at each seek.

For a driver outside `PATH`, pass its executable explicitly:

```sh
DAYDREAM_WEBKIT_WEBDRIVER=/path/to/WebKitWebDriver COREPACK_HOME=/tmp/corepack corepack yarn test:e2e:native
```

Tauri's [native WebDriver guidance](https://v2.tauri.app/develop/tests/webdriver/)
and installed Tauri/Wry source define the existing automation boundary. This
suite uses the driver's native endpoint directly and adds no app plugin or IPC
mock. On the validation host, signed Arch `webkitgtk-6.0` 2.52.6-1 supplied the
standalone driver matching installed GTK3 WebKitGTK 2.52.6. Its dependencies all
resolved to installed libraries. The package and license were extracted under
`/tmp` without a host installation; [official package contents](https://archlinux.org/packages/extra/x86_64/webkitgtk-6.0/files/)
identify the driver. The verified validation package SHA256 was
`1e50283612a5f835d5c505fb7735d2f9cb83dd5d75a9d76328efb68a98a8ee67`;
signature key `83BC8889351B5DEBBB68416EB8AC08600F108CDF` belongs to Arch packager
Jan Alexander Steffens. Retain the distro license with any extracted driver.

The launcher creates private XDG directories under `/tmp`, starts a private D-Bus
session, and enables the existing synthetic capture boundary. It never uses
ambient recordings or devices. Before capture, fixture setup uses public native
settings commands to disable window activity, OCR and automatic processing;
desktop audio and microphone are asserted off. Ambient display variables are
cleared. A test-display preload shim corrects Broadway's
unset DPI sentinel to 96; positive DPI and production assets remain untouched.
Before capture, the journey atomically replaces its isolated package executable
with a fresh copy. Start and Resume must still launch the running executable's
media worker, even though its original inode has been unlinked.
Each phase starts a fresh Broadway display and native process while retaining the
same synthetic data/config/cache/state. This avoids GTK Broadway's disconnect
assertion when reusing a display across process restarts. All owned processes are
stopped and reaped. Synthetic artifacts are retained in the printed directory for
review: per-phase native/display logs, screenshots, asset/input diagnostics and
JSON story reports. Remove that directory when no longer needed.

Covered stories: local consent defaults, preference autosave through navigation,
reusable model capability setup and saving without enabling remote processing,
visible synthetic provider failure, correlated native/frontend file diagnostics
without provider bodies, and successful text/vision retry,
persistence across actual native process restart, reset cancellation and
confirmation, empty Library/Search/Processing navigation, and synthetic screen
capture start/pause/resume/stop through the real native writer. The existing fake
capture adapter supplies synthetic frames at the configured source-output
boundary; portal/device delivery and GPU transforms are outside this suite.
Desktop audio and microphone remain off.
The resulting segments are checked on disk and opened through Library in Studio;
the actual WebKit video must load a decoded frame, play with advancing media time,
and pause. Native command reads corroborate
visible state after user actions. Device permissions, live portals,
microphones, live model authentication and populated Recall workflows require
their separate native integration and browser stories; this suite does not claim
those boundaries.
