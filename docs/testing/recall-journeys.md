# Recall / Library / router QA inventory

Synthetic fixtures only. New tests replace Tauri invoke/listen and time while
retaining real screens, Zustand actions, client decoders, AppShell and TanStack routes.

| Story | Controls / state | Existing proof | Added proof |
| --- | --- | --- | --- |
| LIB-01 | Days default; newest unreviewed past day; Today across midnight; empty CTA; visible cards/status/counts | LibraryScreen browser first4 cases, local-day hook | NAV-01 actual root redirect and Library→Day |
| LIB-02 | Recordings list/grid; selection checkbox/Ctrl/Shift; locked/missing play/delete; bulk summary; refresh/retry | LibraryScreen browser list, empty, error, selection and locked cases | Existing coverage reused |
| LIB-03 | Row play/double-click; rename title/blank/reset/cancel; delete confirm/cancel; bulk delete | LibraryScreen browser row/context actions | Native capture→Library/Studio owner validates real native media |
| LIB-04 | Privacy-aware thumbnail preparing/missing/late response; source eligibility revocation | LibraryScreen thumbnail stale-callback test, Rust eligibility cases | Existing coverage reused |
| REC-01 | Session editor toggle; title/category/local correction; failed save preserves draft; successful reload; feedback Helpful/Needs work/comment/save | Prior Recall failed feedback case | New routed session/all-rating/save/reload journey |
| REC-02 | Accept suggestion; task edit/title/notes/save/error; done/reopen/dismiss; reload; privacy withholding and preserved decision | Prior accept/done/review and dismiss suggestion cases | New complete task state/draft/reload/hide-unhide chain |
| REC-03 | Generation unconfigured/disabled/eligible/zero evidence; provider setup link; regenerate; reviewed/unreviewed fingerprint | Native generation/retry/privacy tests and prior Day reviewed case | New generation blockers + regeneration + review toggle |
| REC-04 | Captured/gap/redacted/missing/uninterpreted coverage; partial/stale/invalidated summary/task withholding; source links do not autoplay | RecallScreen invalidated case, Rust day/timing/privacy tests, Studio source workflow | Existing coverage reused; native populated generation remains model-boundary dependent |
| SRCH-01 | Query blank/nonblank/submit; source all/OCR/desktop/mic; date; semantic/literal mode | Existing Recall query/mode/filter cases | NAV-01 real URL query and history persistence |
| SRCH-02 | Next/previous/reset scope; unavailable/incomplete/empty result; processing/provider links; source provenance | Existing Recall paging/retry/incomplete/hybrid tests | Existing coverage reused |
| SRCH-03 | New scope/native cancellation; privacy metadata revocation; malformed metadata/subscription failure; late response; progress coalescing | Existing Recall race cases and native cancellation/no-upload regressions | Existing coverage reused |
| NAV-01 | `/`→Library Days; sidebar Day/Search/Library; Search query browser history; real route params | Prior route validation unit tests | New actual application router/AppShell journey |
| NAV-02 | Debug-disabled direct Diagnostics; hidden sidebar item; Open Settings → General; no diagnostic read | Prior standalone Diagnostics test | New actual-router privacy/navigation journey |
| NAV-03 | Appearance/light-dark/sidebar rail/mobile Sheet/keyboard/zoom; Capture/Settings/Processing/Notifications destinations | AppShell accessibility, Settings and native UI owners | Cross-owner coverage; no duplicated fixture screens |

## Risk / acceptance boundaries

- Browser cases prove IPC payloads, decoder failures, real routing and visible control transitions; they do not prove SQL/permission/model inference.
- The redesign baseline passed 490 native tests with three explicit benchmarks
  ignored. Source/privacy/retry and persisted generation/task scenarios use real storage.
- Native standalone/full-search/model proofs already recorded by owners. Current native UI harness owns actual Settings/Capture→Library/Studio; no debug data injection or real user database.
- New browser test file: apps/desktop/src/features/recall/testing/recall-review-journeys.browser.test.tsx (six multi-step stories). Initial existing/new combined run:30/30 passed, with an unsuppressed TanStack/React lazy-resume console error.
- A passthrough console regression established red proof: NAV-01 failed, other5 stories passed. Pinned TanStack lazyRouteComponent skipped React.use after its import resolved; standard React.lazy named-export adapters now preserve route APIs/code splitting without that invalid resume.
- Final browser gate:30/30 across the new6 journeys plus existing Recall/Library cases; zero console warnings/errors in /tmp/daydream-route-lazy-green.log. Desktop typecheck, owned lint and diff check pass. Red proof: /tmp/daydream-route-lazy-red.log. Source frozen.
- Browser/IPC tests do not prove native SQL mutations. Native persistence, model
  and media coverage is recorded separately in the user-story validation record.
