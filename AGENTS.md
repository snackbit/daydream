# Repository instructions

Daydream is a Linux-first, local-first desktop recall assistant. It records
private screen, audio, activity and window context so users can review their day.
Read [README.md](README.md) for implemented capabilities and current limits.
Roadmap documents do not authorize additional product scope.

Use plain, direct language. Keep changes scoped, solve the owning cause, and
preserve collaborators' changes. Explicit user instructions take precedence.
Do not create commits or delegate to subagents unless the user requests it.

## Repository map and scoped guidance

| Location | Ownership |
| --- | --- |
| apps/desktop/src | React UI, TanStack Router, Zustand, typed IPC clients; follow [frontend instructions](apps/desktop/src/AGENTS.md). |
| apps/desktop/src-tauri | Rust capture, playback, processing, settings, SQLite; follow [native instructions](apps/desktop/src-tauri/AGENTS.md). |
| packages/shared | Frontend/native TypeScript contracts, grouped by domain. |
| packages/ffmpeg | Development FFmpeg resolver using ffmpeg-static. |
| packages/eslint-config, packages/tsconfig | Shared lint, formatting and strict compiler policy. |

Read scoped instructions before editing that area. Generated output and vendored
code retain their own conventions; change their source/generator or documented
upstream boundary when the task requires it.

## Privacy and product invariants

- Screen recordings, audio, OCR, transcripts, window titles/inventory, input
  activity, prompts, embeddings and summaries are highly sensitive personal data.
- Captured or derived content MUST NOT leave the machine unless the user explicitly
  enables the clearly labeled remote capability. Preserve local defaults.
- MUST NOT log private content or model prompts by default. Use synthetic fixtures;
  never put real recordings, secrets or captured text in tests, public artifacts or
  troubleshooting output. Local audit metadata should describe operations without
  revealing content.
- Recording MUST remain visible and user-controlled. Document native screen,
  microphone, accessibility/input and window-metadata permissions when introduced.
- Capture and processing remain separate. Processing SHOULD be bounded, resumable
  and deterministic where practical; background failures must not silently lose
  authorized recordings.
- Redaction eligibility MUST apply before processing, remote calls, search and
  result display. Changed redactions invalidate affected derived data. Distinguish
  reversible hide/unhide from physical deletion; deleting a recording must remove
  its media and derived artifacts. Follow the active deletion contract and state
  physical-media limitations honestly.
- Annotations, tasks, summaries and search results MUST retain timestamp/source
  provenance. Validate model-generated structured facts at runtime.
- Keep native integrations, model runners, files and indexes behind narrow adapters.
  SQLite belongs to Rust; React MUST use typed Tauri commands.

## Engineering and ownership

- Verify assumptions against source, callers, tests, manifests or official docs
  before depending on them. Keep domain logic deterministic and effects at edges.
- Preserve public hooks, routes, shared exports, Tauri commands/events, payloads and
  storage compatibility unless the task authorizes a change. Migrations are explicit
  and tested. Shared IPC edits require checking both Rust and TypeScript consumers.
- Prefer existing APIs and straightforward logic. New abstractions must remove
  actual duplication or ownership confusion, not create speculative extension points.
- Before adding substantial behavior, inspect branching, lifecycle/race ownership,
  mixed concerns and hot-path cost. Review production files near 400 physical lines;
  files over 500 need a clear ownership justification or a focused split. ESLint's
  separate configured warning is 600 nonblank/noncomment lines.
- Split by responsibility, preserve behavior first, and move tests with their owner.
  Keep stores, shared contracts and Rust commands from becoming catch-all modules.
  Do not suppress lint/dependency warnings to hide design problems.
- Identify hot paths before refactoring: capture callbacks, processing queues,
  timeline/canvas/playback, preview frames and storage scans. Avoid unnecessary
  cloning, repeated large-array filtering/sorting and unstable dense-tree inputs.
- Record active major cleanups and verification in
  [docs/refactoring-tasks.md](docs/refactoring-tasks.md). Keep only open work there
  and remove resolved entries. Do not mark a split complete until its relevant
  checks pass.

## Tools, dependencies and verification

- MUST use the pinned Yarn through Corepack for JavaScript tooling. Use
  COREPACK_HOME=/tmp/corepack with corepack yarn in constrained environments.
  Use yarn install --immutable for lockfile verification.
- Before npm dependency additions/upgrades, use npm view or official metadata;
  inspect engine/peer compatibility and release notes. Update coupled families
  together. Generate locks with the package manager, never hand-edit them.
- Native/model/binary upgrades require official documentation, checksum/license and
  packaging review when relevant. Keep upgrade-sensitive integrations behind adapters.
- Use [docs/agent-validation.md](docs/agent-validation.md) for exact
  scoped commands, full gates, test prerequisites and resource limits. Run focused
  checks during edits, then one relevant final gate after changes settle.
- Format/autofix once per coherent owned edit batch, then inspect the diff. Do not
  run broad rewriting checks over collaborators' unfinished work.
- Tests MUST protect scenarios and owning invariants. Critical UI needs visible
  user-level workflow proof; privacy/native/stateful changes need failure coverage.
  Mock actual external boundaries, not the behavior being tested. Static prose does
  not need content-snapshot tests.
- Report commands/results, scenario coverage, manual checks performed or skipped,
  compatibility changes and material residual risks. Do not claim unrun checks pass.

## Read on demand

- [UI quality runbook](.agents/runbooks/ui-quality-and-scenario-testing.md):
  user-facing workflow, accessibility and manual regression work.
- [Planning runbook](.agents/runbooks/planning.md): use for explicitly requested
  plans. Keep working decisions in the conversation unless the user requests a file.
- [Protected-window contract](docs/protected-window-recording-rules.md): protection
  matching, automatic pause/resume, metadata failures or protection redactions.
