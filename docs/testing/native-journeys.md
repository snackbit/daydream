# Running-app native journeys

These tests launch the actual custom-protocol Tauri app. Commands, SQLite,
migrations, media writing and WebKit playback remain real. The existing fake
capture adapter supplies synthetic monitor pixels; audio devices stay off.

## Commands and isolation

`yarn test:e2e:native` builds fresh assets and runs all three phases.
`yarn test:e2e` also runs JavaScript and native gates. See the
[runner README](../../apps/desktop/scripts/native-e2e/README.md) for prerequisites
and explicit validated-build reuse during iteration.

Keyboard requires a WebKitWebDriver matching the installed WebKitGTK protocol.
The runner resolves it from `DAYDREAM_WEBKIT_WEBDRIVER` or `PATH` and fails clearly
if absent; it never downloads that prerequisite automatically.

Each run uses private XDG directories and a private D-Bus session. Process
restart preserves those same data directories. Each phase gets a fresh GTK
Broadway display; all owned processes are stopped and reaped. Missing tools,
failed assertions and stale embedded assets fail the command. Default execution
never silently skips a phase.

Before capture, public native fixture settings disable window activity, OCR and
automatic processing. Source IDs must identify fake monitors and audio IDs must
be null. Defaults/remote consent are checked before fixture changes. No ambient
capture, user database, release hooks or model download is used.

## Story matrix

| Story | User action and native proof |
| --- | --- |
| NATIVE-CURRENT-FRONTEND-ASSET-IDENTITY | Embedded initial/lazy JS and CSS match current build filenames in every phase. |
| NATIVE-LOCAL-CONSENT-DEFAULTS | Fresh settings use local providers and disabled text generation. |
| NATIVE-SETTINGS-SAVE | A real checkbox click and Save persist the expert preference. |
| NATIVE-SETTINGS-DIRTY-STAY-DISCARD | Stay retains the dirty screen; Discard navigates without writing the draft. |
| NATIVE-NAVIGATION-EMPTY-LIBRARY-SEARCH-PROCESSING | Sidebar clicks load the actual routes and empty states. |
| NATIVE-SYNTHETIC-CAPTURE-SAVE-START-PAUSE-RESUME-STOP | Save source setup, then record/pause/resume/stop through public handlers and the real writer. |
| NATIVE-RECORDED-MEDIA-LIBRARY-STUDIO | Finalized files exist under isolated app data; Library opens their decoded video in Studio. |
| NATIVE-STUDIO-PLAY-TIME-ADVANCE-PAUSE | Trusted Play/Pause clicks advance and stop actual media time. |
| NATIVE-SETTINGS-RESTART | The saved preference survives a real application process restart. |
| NATIVE-SETTINGS-RESET-CANCEL-CONFIRM | Cancel preserves settings; explicit confirmation restores defaults. |
| NATIVE-SEARCH-TYPE-SUBMIT-EMPTY-RESULT | Trusted keyboard input submits the query to real native search and displays an eligible empty result. |

There are 11 unique stories and 13 phase entries: asset identity repeats across
initial, restart and keyboard phases.

## Reproduced application and adapter bugs

The adapter originally emitted a composed physical-source frame even when the
GUI configured independent outputs. It now emits each configured scene-source
UUID and output dimension, preserving the composed fallback. A public-command
regression failed before the fix, then both modes produced valid media.

The adapter also capped its interval at 100 ms. At the GUI's 0.2 FPS cadence this
advanced writer timestamps into the future and created invalid fragment starts.
It now honors configured cadence with interruptible shutdown. A sparse public
lifecycle regression reproduced the SQLite timestamp constraint before the fix.
Three composed/independent/sparse scenarios pass and verify output dimensions
and fragment starts no later than recording stop.

The GUI exposed enabled Resume during pause finalization although the store
correctly refused it. Trusted mouse traces proved the click reached that control.
Capture now shares the store/global lifecycle lock; browser regression coverage
proves disabled controls until completion, then a real Resume command.

## Headless driver constraints

Broadway reports unset DPI on this host. A test-only GDK shim forwards valid DPI
and maps nonpositive values to 96. An inert temporary pixel marker calibrates
WebView origin inside GTK decorations and is removed before user stories.
Clicks wait for visible/enabled targets and acknowledge trusted native events.

GTK display disconnects require a fresh display per phase. Broadway's keyboard
transport intermittently delivered down/up without text input, including in the
combined gate. Input-method and rendering waits did not make it reliable.

Keyboard now uses official WebKitWebDriver W3C actions and Tauri/Wry's existing
automation environment flag. Its separate TCP inspector controls the same real
WebView; initial/restart retain the pointer/HTTP inspector. Three isolated
repetitions and the fresh default native suite passed with exact committed text,
key counts and trusted events. The driver never retries a character, edits input
values or substitutes app logic. GTK simple IM and `C.UTF-8` standardize the
fixture; they are not credited as the fix.

Validation used the signed Arch `webkitgtk-6.0` 2.52.6-1 package, matching the
installed GTK3 WebKitGTK version. Only its 1.7 MB driver was extracted under
`/tmp/daydream-webdriver`; dependencies resolved to installed libraries and its
license was retained. The detached signature was checked against a copy of the
installed Arch keyring (heftig `83BC8889351B5DEBBB68416EB8AC08600F108CDF`).
Package SHA256: `1e50283612a5f835d5c505fb7735d2f9cb83dd5d75a9d76328efb68a98a8ee67`.
No system installation or app hooks were added.

## Verification and limits

The native suite passed all 11 unique stories after fresh builds. Latest standalone
evidence: `/tmp/daydream-native-webdriver-final.log` and
`/tmp/daydream-native-e2e.yFNTsA/`. The combined gate is recorded in the
[main validation record](../user-story-validation.md).

Final combined gate exits 0 after fresh builds and all default phases. Evidence:
`/tmp/daydream-production-e2e-verified.log`, with reports and screenshots in
`/tmp/daydream-native-e2e.3eXHeR/`.

Real portal/PipeWire/microphone permissions, compositor/device delivery, native
zoom and distribution installers remain operator checks. Populated Recall,
privacy, model and provider behavior have separate browser/native/model proofs.
These bounded native journeys do not claim every hardware combination.
