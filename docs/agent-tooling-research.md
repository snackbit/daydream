# Agent tooling and dependency research: 2026-10-05

Research started during the read-only master review of clean main
6527f5fb14435f52f2bd880dde0e9635eaa50506. Implementation followed review completion
and task authorization. Original findings remain in [master review](reviews/master-review.md).

## Repository evidence and changes

- Baseline AGENTS.md was 396 lines and said implemented capture, OCR, transcription,
  redaction and playback were absent. README and recorder/AI/playback source showed
  the Linux alpha. Root guidance is now 108 lines of stable invariants and routing;
  README owns capability status. Frontend/native instructions load only for their area.
- Desktop README was a Tauri template. It now describes actual native/frontend loops
  and the limits of Vite pages and mocked Browser Mode tests.
- The 3,282-line refactoring checklist imported a completed cleanup goal and repeated
  old check results. A short active section now precedes labeled completed history.
  It records owned splits and checks without restarting old work.
- New [validation map](agent-validation.md) owns exact commands and budgets.
  README, CONTRIBUTING and runbooks link to it. Unit/browser and Rust fmt/clippy/test
  aliases expose their costs; Cargo/Vitest concurrency is bounded at two.
- Recorder tests no longer repeat unit coverage then silently run all Rust tests.
  Focused recorder/E2E, coverage and full native checks have distinct commands.
- Instructions preserve privacy, IPC/storage compatibility, actual shadcn primitives,
  scenario tests and user control. User-requested studio direction is explicit:
  dense controls, quiet surfaces, restrained color and minimal radius/borders.
- Static guidance has no content-snapshot tests. Existing repo checks validate
  tooling/dependencies. No CI system was added merely because .github was absent.
- node_modules ignore rules now cover directories and shared-worktree symlinks.

## Comparable projects

GitHub shows rounded stars. Observed commit dates establish activity; cached pages
do not prove the latest commit today. All sources are the projects' own repositories.

| Project | Displayed stars | Observed activity | Comparable scope |
| --- | --- | --- | --- |
| [Screenpipe](https://github.com/screenpipe/screenpipe) | 21.8k | [Sep 9, 2026](https://github.com/screenpipe/screenpipe/commits/main/) | Rust, Tauri/React, capture/local AI monorepo |
| [Jan](https://github.com/janhq/jan) | 44.8k | [Sep 4, 2026](https://github.com/janhq/jan/commits/main) | Yarn, React/Tauri, native local models |
| [Spacedrive](https://github.com/spacedriveapp/spacedrive) | 39.1k | [Jul 29, 2026](https://github.com/spacedriveapp/spacedrive/commits/main/) | Rust/Tauri/React storage monorepo; slower recent activity |
| [GitButler](https://github.com/gitbutlerapp/gitbutler) | 21.8k | [Sep 7, 2026](https://github.com/gitbutlerapp/gitbutler/commits/master/) | Rust/Tauri/Svelte plus React app monorepo |
| [Seelen UI](https://github.com/eythaann/Seelen-UI) | 18.0k | [Aug 27, 2026](https://github.com/eythaann/Seelen-UI/commits/master/) | Adjacent Windows-native Tauri monorepo with React/Preact guidance |

[Screenpipe instructions](https://github.com/screenpipe/screenpipe/blob/main/AGENTS.md)
keep root context short, route readers to specific docs, separate browser validation
from native builds and name the native build queue/cache. They call out code excluded
from root Cargo tests. Daydream adopts scoped routing and clear check ownership;
it does not copy Screenpipe's capture or publication policy.
[CLAUDE.md](https://github.com/screenpipe/screenpipe/blob/main/CLAUDE.md) points to the
canonical guide rather than maintaining another rule set.

[GitButler root](https://github.com/gitbutlerapp/gitbutler/blob/master/AGENTS.md) gives
a short map and focused validation. Its [Rust guide](https://github.com/gitbutlerapp/gitbutler/blob/master/crates/AGENTS.md)
names API/lock/compatibility ownership and excludes generated/vendor data. This
supports Daydream's scope-local native invariants, not copying GitButler's migration
or Git-write policy.

[Jan scripts](https://github.com/janhq/jan/blob/main/package.json) separate core,
web and extension test projects. Its [Makefile](https://github.com/janhq/jan/blob/main/Makefile)
uses one resource-stub generator across checks instead of duplicating setup.
Root AGENTS/CLAUDE fetches failed, so no guide contents are inferred.
[Spacedrive guidance](https://github.com/spacedriveapp/spacedrive/blob/main/AGENTS.md)
names Rust-to-TypeScript regeneration and wrong command/async mistakes. Daydream
adopts an explicit cross-boundary checklist; generated IPC adoption remains a
separate architectural task.
[Seelen guidance](https://github.com/eythaann/Seelen-UI/blob/master/AGENTS.md)
distinguishes cargo check, debug binaries and release builds and names lock order.
Daydream adopts cost distinctions, without inventing unverified lock orders.

## JavaScript dependency selection

Official registry metadata was read for all 47 original direct external packages,
including engines, peers and publication times. Yarn's 24-hour quarantine remains
enabled: plugin-react 6.1.2 was too recent, so 6.1.1 was selected. Versions are pinned;
the generated lock was refreshed within parent ranges, without incompatible overrides.

| Dependency | Before | Final |
| --- | --- | --- |
| @fontsource-variable/inter | 5.2.8 | 5.3.0 |
| @tanstack/react-router | 1.170.11 | 1.170.41 |
| @tauri-apps/api | 2.11.0 | 2.12.1 |
| @tailwindcss/vite | 4.3.0 | 4.3.3 |
| @tauri-apps/plugin-opener | 2.5.4 | 2.7.0 |
| radix-ui | 1.4.3 | 1.6.7 |
| lucide-react | 1.17.0 | 1.52.0 |
| react-resizable-panels | 4.11.2 | 4.14.2 |
| react | 19.2.7 | 19.3.0 |
| shadcn | 4.10.0 | Removed |
| tailwind-merge | 3.6.0 | 3.7.0 |
| react-dom | 19.2.7 | 19.3.0 |
| zustand | 5.0.14 | 5.0.15 |
| tailwindcss | 4.3.0 | 4.3.3 |
| @testing-library/dom | 10.4.1 | 10.4.2 |
| @testing-library/react | 16.3.2 | 16.3.3 |
| @types/react | 19.2.16 | 19.3.0 |
| @tauri-apps/cli | 2.11.2 | 2.12.1 |
| @types/react-dom | 19.2.3 | 19.3.0 |
| @vitejs/plugin-react | 6.0.2 | 6.1.1 |
| @vitest/browser-playwright | 4.1.8 | 4.1.11 |
| @vitest/coverage-v8 | 4.1.8 | 4.1.11 |
| @types/node | 25.9.1 | 24.19.1 |
| eslint | 9.39.4 | 9.39.5 |
| vitest | 4.1.8 | 4.1.11 |
| @eslint/js | 9.39.4 | 9.39.5 |
| globals | 17.6.0 | 17.13.0 |
| prettier | 3.8.3 | 3.9.9 |
| playwright | 1.60.0 | 1.63.0 |
| prettier-plugin-tailwindcss | 0.8.0 | 0.8.1 |
| typescript-eslint | 8.60.1 | 8.71.0 |
| vite | 8.0.16 | 8.3.2 |
| tailwindcss-animate | 1.0.7 | Removed; existing tw-animate-css retained |

The unused shadcn generator was installed solely for a CSS import. Real generated
components remain unchanged; nine upstream custom variants from shadcn 4.21.1 now
live in local shadcn-variants.css with MIT attribution. No used accordion/scroll-fade
utilities were found. Removing the CLI eliminated 209 transitive packages and its
unpatched braces path. The [upstream CSS](https://github.com/shadcn-ui/ui/blob/main/packages/shadcn/src/tailwind.css)
and [shadcn animation guidance](https://ui.shadcn.com/docs/tailwind-v4) support keeping
component/helper source local and using tw-animate-css. Future generation still uses
components.json and requires verified safe official tooling/registry source.

Compatibility holds:

- TypeScript stays 6.0.3: latest [typescript-eslint](https://registry.npmjs.org/typescript-eslint/latest)
  declares TypeScript >=4.8.4 <6.1.0.
- ESLint stays 9.39.5: [eslint-plugin-react](https://registry.npmjs.org/eslint-plugin-react/latest)
  excludes ESLint 10 in its peer contract. ESLint 9 is
  [out of support](https://eslint.org/version-support/); this is explicit residual
  maintenance debt, not a claim that it remains supported.
- Vitest stays on patched 4.1.11 with matching browser/coverage packages.
  [Vitest 5](https://vitest.dev/guide/migration/) changes defaults/mock lifecycle;
  the compatible 4 fixes the identified advisories without that migration.
- Node types align to Node 24. Supported Node engines exclude Node 23 as Vitest 4
  does: ^22.12.0 or >=24. Node 24 is the recommended
  [LTS branch](https://nodejs.org/en/about/previous-releases).

## Advisory assessment

The original Yarn audit returned 85 entries, including deprecations. The final
all-workspace recursive audit returns zero security advisories and one ESLint 9
deprecation; Yarn exits 1 for that deprecation. This is not a security all-clear
for native code or vendored binaries.

Patched checks include [Vitest browser file gates](https://github.com/advisories/GHSA-p63j-vcc4-9vmv),
[Vitest mocker traversal](https://github.com/advisories/GHSA-82fw-gwwq-j7x9) and
[node-tar decompression](https://github.com/advisories/GHSA-23hp-3jrh-7fpw).
The [braces advisory](https://github.com/advisories/GHSA-vfj7-8cjw-p6xm) has no patched
release; removing the unused generator eliminated its only dependency path.

OSV batch queries cover every registry package/version in Cargo.lock, including
platform-specific packages. The original 629-package query matched 16 advisory IDs.
The final 631-package query matches three IDs across two packages; two IDs alias
the same GLib issue. This is an OSV assessment, not cargo-audit execution.

| Native package | Before | Final |
| --- | --- | --- |
| tauri | 2.11.2 | 2.12.1 |
| tauri-build | 2.6.2 | 2.7.1 |
| tauri-plugin-opener | 2.5.4 | 2.7.0 |
| rusqlite | 0.40.0 | 0.40.2 |
| ashpd | 0.13.11 | 0.13.13 |
| uuid | 1.23.2 | 1.27.0 |
| zbus | 5.16.0 | 5.19.0 |
| anyhow | 1.0.102 | 1.0.104 |
| event-listener | 5.4.1 | 5.4.2 |
| quinn-proto | 0.11.14 | 0.11.19 |
| rustls | 0.23.40 | 0.23.45 |
| serde_with | 3.20.0 | 3.24.0 |
| plist | 1.9.0 | 1.10.1 |
| wayland-scanner | 0.31.10 | 0.31.11 |
| quick-xml | 0.39.4 | 0.41.0 and 0.42.0 |

The XML owner upgrades remove [quadratic parsing](https://rustsec.org/advisories/RUSTSEC-2026-0194.html)
and [unbounded namespace allocation](https://rustsec.org/advisories/RUSTSEC-2026-0195.html)
without forcing incompatible parser APIs. Updated Tauri removes the unmaintained
Unicode graph. GStreamer 0.24, PipeWire 0.9 and keyring 3 retain their compatible
latest lines; vendored Whisper/GPU/native binary changes were not bundled in.

Remaining native risk: [GLib VariantStrIter unsoundness](https://rustsec.org/advisories/RUSTSEC-2024-0429.html)
(RUSTSEC-2024-0429 / GHSA-wrw7-89jp-8q8g, GitHub Moderate) affects GLib 0.18.5.
Patched >=0.20 cannot satisfy the GTK3 GLib ^0.18 graph. Current
[Wry 0.57 manifest](https://github.com/tauri-apps/wry/blob/wry-v0.57.0/Cargo.toml)
and GTK/WebKit dependencies keep that graph. No maintained 0.18 backport was found
in official crate metadata. Product source and the inspected GTK/GIO/GDK/WebKit
sources have no direct VariantStrIter/array_iter_str calls; complete transitive
runtime reachability is not proven. Do not claim immunity or silence the advisory.
A supported upstream backport or GTK/WebKit stack migration is follow-up work.

[proc-macro-error 1.0.4](https://rustsec.org/advisories/RUSTSEC-2024-0370.html) is
unmaintained informational debt retained by GTK3 macros. It has no fixed release
in that line. No incompatible resolution or large GTK fork was introduced.

Rust minimum is 1.95, already required by locked rusqlite_migration 2.6.0; declaring
it also covers the new [File::try_lock](https://doc.rust-lang.org/std/fs/struct.File.html#method.try_lock)
primitive (stable since 1.89). Version metadata came from
[crates.io](https://crates.io/) and advisories from [OSV](https://osv.dev/)/RustSec.

## Verification and limits

- Immutable Yarn install passed after the final graph change; matching Playwright
  Chromium is installed. Main lint/typecheck and 290 tests passed; production build
  passed with the known eager chunk warning (859 kB / 256 kB gzip).
- Focused privacy/native tests and ownership splits are recorded in
  [master review](reviews/master-review.md). Final native checks passed 349 tests,
  formatting and strict Clippy; 94 focused AI/transcription checks cover finishing
  cancellation/Clippy refinements. Real native hardware remains a manual limit.
- Main/worktree manifests, generated locks, guidance and shared formatter changes
  were synchronized without commits. UI CSS state checks and isolated final gates
  belong to the UI review; no real capture session or packaged binary was validated.
- Advisory logs and official metadata are retained under /tmp during this session.
  Network/approval capacity failures were resolved, not bypassed. Test gates passing
  does not erase the GLib risk or ESLint/proc-macro-error maintenance debt.
