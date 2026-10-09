# Studio control and scenario audit

Synthetic fixtures only. Existing screen suite is reused; new stories live in `features/library/recording-studio-route/stories` and exercise production components/controllers/stores/typed clients, replacing only Tauri IPC, router navigation and native media boundaries.

## Risk matrix

| Workflow | Failure risk | Owning proof |
| --- | --- | --- |
| Rename/delete route mutations | lost draft, hidden error, duplicate writes, navigation before success | STU-01/STU-02 Browser Mode, typed native invoke boundary |
| Source/global audio controls | global unmute overrides individual choice, volume does not reach source | STU-03 Browser Mode, actual media elements with playback boundary fake |
| Privacy editing | stale OCR/transcript/activity visible after cuts, unsafe playback | existing screen/OCR/evidence browser workflows plus native privacy tests |
| Timeline navigation | wrong coordinate/segment, sticky selections, wheel conflict | existing screen workflows and store/math units |

## Complete production-control inventory

References below abbreviate `RecordingStudioScreen.browser.test.tsx` as Screen, `inspector/ocr/ocr-inspector.browser.test.tsx` as OCR, `studio-recall-evidence.browser.test.tsx` as Evidence.

| Controls / states | Existing scenario proof / new story | Gaps |
| --- | --- | --- |
| Toolbar Process AI, Starting, Processing, Processed, Retry AI; active chunks despite successful parent | Screen starts processing; processed state; active chunks; event updates; legacy transcription failure configuration | real model setup/device execution belongs native/manual |
| Toolbar Rename; title input; max length; trimmed/blank title; Save/Cancel; rejected save and retry | Screen rename success; STU-01 cancel/120 allowed/121 blocked/rejection/draft retention/retry/blank reset | PASS |
| Toolbar Delete; Cancel; rejected deletion; retry; pending duplicate guard; route return | Screen broken recording delete; STU-02 | PASS |
| Toolbar Refresh; failed recording load; unavailable/missing files; playback retry | Screen refresh reconciles tracks/clips; missing placeholders; session races | debug workflow below |
| Debug gate; debug data dialog; Copy JSON; copy failure | STU-04 opt-in gate/no read, failed read reopen recovery, clipboard denial then successful copy | PASS |
| Video source combobox; independent streams; Grid view | Screen source/grid isolation and scoped source masks; Evidence exact microphone alias | real native stream encoding belongs native harness |
| Play/Pause; Space; editable-target protection; native play rejection; Retry playback; replay after end | STU-13 native media rejection then Retry/canplay on exact same source; Screen keyboard playback/keys; media rejection; replay; pause-before-end intent | new retry story pending |
| Playback position slider; ruler/lane seek; previews; scrub commit; paused seek; canplay resumption | Screen paused seek, canplay scrub, thumbnails, ruler and lower activity lane seek | existing user flows |
| Skip back/forward; paused-gap Play now; timed gap transitions | STU-05 repeated direct skip clamps both ends without autoplay; Screen Play now, gap waits, back-to-back segments | PASS |
| Frame punctuation keyboard navigation, actual saved frames, text-edit protection | Screen frame keys; source scoped adjacent native frame | existing user flow |
| Fullscreen | Screen fullscreen player-surface regression | host fullscreen/manual remains |
| Global Mute/Unmute, per-audio-source Mute/Unmute, Volume keyboard | STU-03 combines independent source/global states and Inspector rename | PASS |
| Seekbar cut markers; video masks; audio cuts; auto-skip countdown; manual skip and paused cancellation | STU-12 direct manual skip reaches exact eligibility boundary without autoplay; Screen masks before timeupdate, scoped seekbar, auto-skip; `use-redaction-auto-skip.browser` timing cases | native redaction tests own physical delivery |
| Track selection label/body; ctrl add; clear plain click; multi-track Inspector | Screen lane body/add/clear/per-lane inspector | existing user flows |
| Range ruler drag all lanes; lane marquee; ctrl additive drag; ctrl-shift subtract; overlap union | Screen respective range scenarios; timeline selection units | existing user flows |
| Zoom in/out/reset value; Ctrl-wheel; Shift horizontal wheel line/page/bounds; gesture priority | Screen zoom ruler and each wheel permutation | existing user flows |
| Lane reorder handle | STU-06 keyboard reorder commits visible row order and Escape cancels subsequent drag without autoplay; Screen handle availability; store lane order units | PASS |
| Inspector track name; color popover selection; clip title | Screen lane edits and clip rename; STU-03 renamed audio mute | existing user flows |
| Create range redactions; Unredact selected mask or overlapping ranges; grouped redaction resizing both edges/error retry | Screen create/delete/overlap; STU-07 range create/unhide failure/retry, STU-08 mask failure/retry, STU-09 toolbar create/unhide recovery, STU-10 duplicate pending write/later selection/old exact native input, STU-11 actual-screen old mask completion preserves new selected mask; `redactions/grouped-redactions.browser` atomic resize and rejection | PASS; native privacy owner separate |
| Selected-frame inspector; Jump to source; Clear frame; precise subsecond offsets | Screen video marker details; inspector.browser subsecond seek | existing user flows |
| Activity event and interval inspection; Jump to source; Clear selected source; unavailable mouse/keyboard permissions | Screen focused marker/intervals/source provenance/permission-unavailable | focused-window strips are read-only, no user annotation CRUD exists |
| Transcript per-source results, no-speech vs text, failed/provider unavailable state, retry/configure link | Screen transcript segment offsets, desktop-vs-mic separation and privacy cut; inspector.browser | existing user flows |
| OCR Text/Changes/Details; reading modal; literal text find; Copy text success/failure; uncertain words; Jump source; history revision selection; Load more | OCR dense frame find/copy/source; changed digit/grouped support/pagination; literal Vision provenance | existing typed IPC user flows |
| OCR read failure Retry, processing failure Retry, Reprocess pinned configuration; job phases/no frame/empty success | OCR failed Vision retry, processing gaps, pre-frame events; Screen local/remote config immutability permutations | actual OCR runners separate native/manual |
| OCR Debug raw detail gate | OCR literal Vision/debug | existing typed IPC user flow |
| Recall evidence source navigation; Jump; Hide interval; error/retry; unavailable Check again | Screen exact source seek/no autoplay/hide failure/retry; Evidence validates subscription before private read and handles stale/malformed privacy events | native evidence privacy contracts separate |
| Inspector show/hide persistent edits/playback; desktop/narrow adaptive layout | Screen Inspector retention and desktop workspace; capture/layout owned tests shared | real WebKit/packaged manual remains |

## Validation

Root-scheduled red Browser Mode run observed STU-01/STU-02 missing in-modal errors; rename also produced an uncaught rejected promise. STU-03 exercised volume/global/source mute successfully, then fixture lacked desktop viewport for Inspector. Corrected fixture to desktop and real stylesheet. Production fixes catch rename rejection while retaining draft, and await delete with visible error/retry and disabled pending actions. Added missing Debug/direct skip/keyboard reorder workflows. Subsequent browser proof passed STU-01/STU-03/STU-04/STU-05; pending fixtures corrected physical Enter DnD activation and disabled-click simulation to actual double-click intent.

Privacy red-first batch observed each missing error state and five uncaught privacy rejections in range Inspector, mask Inspector and timeline toolbar; pending toolbar allowed a repeat request. Shared privacy mutation hook now owns visible caught errors/pending guards. Range success compares recording + submitted native range signature before clearing, avoiding clearing a newer selection even when native detail reconciliation recreates arrays. Controls announce pending writes. STU-11 drives the actual screen/controllers through pending mask completion and switching masks.

Targeted owned-file ESLint autofix and diff whitespace check passed. Latest desktop typecheck passed before final STU-11 addition. Timeline store unit suite passed 16/16. Final 15-case browser batch passed 12/15: all ten route/privacy cases pass; audio/skip base controls pass; three remaining failures diagnosed as test media URLs producing genuine browser decode errors over test controls, plus browser iframe keyboard focus. Fixed test boundaries with a 7,062-byte ten-second black/silent WebM generated only from lavfi (documented generator command), served by Vite using a test-only asset import. Reorder now physically clicks its handle before native keyboard input. Pending narrow five-case audio rerun. Logs `/tmp/daydream-studio-red.log`, `/tmp/daydream-studio-redactions-red.log`, `/tmp/daydream-studio-green.log`, `/tmp/daydream-studio-final-browser.log`, `/tmp/daydream-studio-lint.log`, `/tmp/daydream-studio-typecheck.log`, `/tmp/daydream-studio-unit.log`.

Existing TrackTimeline layout owner remains 424 physical lines (down from 438); focused mutation ownership moved to a 74-line redactions hook. Existing flat Studio folder density remains pre-existing debt; the new implementation lives in the focused redactions folder, and all new stories live in a focused stories folder.

## Explicit limits

Final media boundary repair proved five of six audio cases. Keyboard reorder/cancel, source/global mute, direct skip bounds, manual redacted skip and preparation Retry loading all pass with valid synthetic media. STU-13 reached the correct prefixed native error; corrected selector from whole unprefixed text to the injected boundary error substring. Sole remaining check is STU-13-only rerun (root slot/final gate). Combined 15/16 new cases currently proven. Final desktop typecheck passed including all sixteen stories and the test asset; focused store units 16/16 and targeted lint/diff checks pass. Logs `/tmp/daydream-studio-audio-final.log` and `/tmp/daydream-studio-typecheck-final.log`.

Browser fake boundaries do not prove Linux portal/device capture, WebKit codecs, FFmpeg streaming, actual OCR/VLM runners or packaged fullscreen. No ambient media or existing database opened. No new dependencies or commits.

Final combined gate: all 16 added Studio stories pass, including the corrected
STU-13 selector. The running native app independently verifies decoded media and
actual Play/time advance/Pause. Full evidence is recorded in
[production user stories](../user-story-validation.md).
